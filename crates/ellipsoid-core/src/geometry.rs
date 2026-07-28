//! Ellipsoid tessellation — port of `computeGeometry` in `app/utils/ellipsoid.js`.
//!
//! Faithful to the original including its quirks (see `RUST_CONVERSION_PLAN.md`
//! §8). Behaviour is fixed only after the parity harness is green, and then in
//! separate commits.

// Index loops are kept so this reads line-by-line against the JavaScript. That
// correspondence is the point of a bug-for-bug port: the index arithmetic here
// is load-bearing and subtle, and rewriting it as iterator chains would make
// divergences harder to spot in review, not easier.
#![allow(clippy::needless_range_loop)]

use glam::DVec3;
use std::f64::consts::PI;

use crate::input::EllipsoidInput;

/// A tessellated ellipsoid surface.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// Surface points indexed `[phi][theta]`.
    ///
    /// Outer length is `phi_divisions + 1` and inner length is
    /// `theta_divisions + 1`. The last phi row duplicates the first so edges
    /// close around the circumference.
    pub points: Vec<Vec<DVec3>>,

    /// Longitudinal divisions actually used, after clamping to a minimum of 3.
    pub phi_divisions: usize,

    /// Latitudinal divisions actually used.
    ///
    /// Larger than the requested value: the original inserts a row at
    /// `theta == 0` when the range spans the equator, and one more for each of
    /// `h_middle`, `h_top`, and `h_bottom` that is active.
    pub theta_divisions: usize,

    /// Index of the widest theta row. The cylindrical projection unrolls
    /// outward from here. Was `indexWide`.
    pub widest_row: usize,
}

/// Coerce NaN to zero, as the original's `isNaN(x) ? 0 : x` guards did.
fn or_zero(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v }
}

/// Tessellate the ellipsoid described by `input`.
///
/// # Panics
///
/// If the geometry is degenerate enough that the scan for the equator or the
/// widest row runs off the end of a row. The original threw a `TypeError` in
/// the same situations; no golden case reaches them.
pub fn compute_geometry(input: &EllipsoidInput) -> Geometry {
    // ---------------------------------------------------------------------
    // Normalise inputs. Every fudge constant here is the original's.
    // ---------------------------------------------------------------------
    let a = or_zero(input.a);
    let b = or_zero(input.b);
    let c = or_zero(input.c);

    // -90 and +90 are pulled in by a degree to avoid degenerate poles.
    let theta_min_deg = if input.theta_min.is_nan() {
        -90.0
    } else {
        input.theta_min
    };
    let theta_min = if theta_min_deg == -90.0 {
        -89.0 * PI / 180.0
    } else {
        theta_min_deg * PI / 180.0
    };

    let theta_max_deg = if input.theta_max.is_nan() {
        90.0
    } else {
        input.theta_max
    };
    let theta_max = if theta_max_deg == 90.0 {
        89.0 * PI / 180.0
    } else {
        theta_max_deg * PI / 180.0
    };

    let h_top = or_zero(input.h_top);
    let h_top = if theta_max <= 0.0 && h_top == 0.0 {
        0.001
    } else {
        h_top
    };

    // Cylindrical projection divides by h_middle, so zero becomes epsilon.
    let h_middle = or_zero(input.h_middle);
    let h_middle = if h_middle == 0.0 { 0.001 } else { h_middle };

    let h_bottom = or_zero(input.h_bottom);
    let h_top_fraction = or_zero(input.h_top_fraction);
    let h_top_shift = or_zero(input.h_top_shift);

    let phi_divisions = input.phi_divisions.max(3);
    let mut theta_divisions = input.theta_divisions.max(3);

    // ---------------------------------------------------------------------
    // Angle arrays
    // ---------------------------------------------------------------------
    let step = (theta_max - theta_min) / theta_divisions as f64;
    let mut thetas: Vec<f64> = (0..=theta_divisions)
        .map(|i| theta_min + i as f64 * step)
        .collect();

    // Force an exact 0 into the array when the range spans the equator; the
    // widest-row search and the h_middle split both key off it.
    if theta_max > 0.0 && theta_min < 0.0 {
        let mut idx = 0;
        while thetas[idx] < 0.0 {
            idx += 1;
        }
        if thetas[idx] != 0.0 {
            thetas.insert(idx, 0.0);
            theta_divisions += 1;
        }
    }

    let phis: Vec<f64> = (0..=phi_divisions)
        .map(|i| -PI + i as f64 * (2.0 * PI / phi_divisions as f64))
        .collect();

    // ---------------------------------------------------------------------
    // Surface points
    // ---------------------------------------------------------------------
    let mut points: Vec<Vec<DVec3>> = Vec::with_capacity(phi_divisions + 1);
    for &phi in phis.iter().take(phi_divisions + 1) {
        let mut row = Vec::with_capacity(theta_divisions + 1);
        for &theta in thetas.iter().take(theta_divisions + 1) {
            row.push(DVec3::new(
                a * theta.cos() * phi.cos(),
                b * theta.cos() * phi.sin(),
                c * theta.sin(),
            ));
        }
        points.push(row);
    }

    // ---------------------------------------------------------------------
    // Added height
    // ---------------------------------------------------------------------

    // h_middle: duplicate the equator row, then push the halves apart.
    if h_middle != 0.0 && theta_max > 0.0 && theta_min < 0.0 {
        let mut insert = 0;
        while points[0][insert].z < 0.0 {
            insert += 1;
        }
        theta_divisions += 1;
        for row in points.iter_mut() {
            let duplicate = row[insert];
            row.insert(insert, duplicate);
            for it in 0..=insert {
                row[it].z -= h_middle / 2.0;
            }
            for it in (insert + 1)..=theta_divisions {
                row[it].z += h_middle / 2.0;
            }
        }
    }

    // h_top: append a scaled, shifted copy of the top ring, raised by h_top.
    if h_top != 0.0 {
        let insert = theta_divisions;
        for row in points.iter_mut() {
            let base = row[insert];
            row.push(DVec3::new(
                base.x * h_top_fraction + h_top_shift,
                base.y * h_top_fraction,
                base.z + h_top,
            ));
        }
        theta_divisions += 1;
    }

    // h_bottom: prepend a copy of the bottom ring, dropped by h_bottom.
    if h_bottom != 0.0 {
        for row in points.iter_mut() {
            let base = row[0];
            row.insert(0, DVec3::new(base.x, base.y, base.z - h_bottom));
        }
        theta_divisions += 1;
    }

    // ---------------------------------------------------------------------
    // Widest row
    // ---------------------------------------------------------------------
    let widest_row = if theta_min >= 0.0 {
        0
    } else if theta_max <= 0.0 {
        theta_divisions - 1
    } else {
        let mut i = 0;
        while points[0][i].z < 0.0 {
            i += 1;
        }
        // The original computed `i - 1` unguarded, yielding -1 and corrupting
        // everything downstream if the first point were already at or above
        // the equator. Reaching theta_min < 0 < theta_max guarantees i > 0, so
        // this asserts rather than silently wrapping.
        assert!(
            i > 0,
            "widest-row scan found no point below the equator despite \
             theta_min < 0 < theta_max"
        );
        i - 1
    };

    Geometry {
        points,
        phi_divisions,
        theta_divisions,
        widest_row,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_dimensions_follow_the_division_counts() {
        let g = compute_geometry(&EllipsoidInput::default());
        assert_eq!(g.points.len(), g.phi_divisions + 1);
        for row in &g.points {
            assert_eq!(row.len(), g.theta_divisions + 1);
        }
    }

    #[test]
    fn defaults_add_three_theta_rows() {
        // 16 requested, +1 for the inserted equator, +1 for h_middle,
        // +1 for h_bottom. h_top is 0 so it adds nothing.
        let g = compute_geometry(&EllipsoidInput::default());
        assert_eq!(g.theta_divisions, 19);
    }

    #[test]
    fn divisions_are_clamped_to_three() {
        let input = EllipsoidInput {
            phi_divisions: 1,
            theta_divisions: 0,
            ..Default::default()
        };
        let g = compute_geometry(&input);
        assert_eq!(g.phi_divisions, 3);
        assert!(g.theta_divisions >= 3);
    }

    #[test]
    fn phi_ring_closes_on_itself() {
        // The last phi row duplicates the first (phi runs -PI..=PI), which the
        // edge construction in `flatten` relies on.
        let g = compute_geometry(&EllipsoidInput::default());
        let first = &g.points[0];
        let last = &g.points[g.phi_divisions];
        for (p, q) in first.iter().zip(last.iter()) {
            assert!((p.x - q.x).abs() < 1e-12, "{p:?} vs {q:?}");
            assert!((p.y - q.y).abs() < 1e-12, "{p:?} vs {q:?}");
            assert!((p.z - q.z).abs() < 1e-12, "{p:?} vs {q:?}");
        }
    }

    #[test]
    fn nan_inputs_degrade_to_zero_rather_than_propagating() {
        let input = EllipsoidInput {
            a: f64::NAN,
            h_top_shift: f64::NAN,
            ..Default::default()
        };
        let g = compute_geometry(&input);
        assert!(
            g.points.iter().flatten().all(|p| p.is_finite()),
            "NaN leaked into the surface"
        );
    }
}
