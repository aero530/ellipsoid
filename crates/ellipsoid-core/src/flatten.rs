//! Panel unrolling — port of `computeFlatGeometry` in `app/utils/ellipsoid.js`.
//!
//! Each panel strip is uncurled by walking along it and rotating everything
//! below (or above) the current rung about that rung's edge, by the angle
//! between adjacent panel planes. Spherical unrolls from the top; cylindrical
//! unrolls outward from the widest row, then unwraps the resulting cylinder.
//!
//! This is a bug-for-bug port. Four things here look wrong and are meant to be
//! (see `RUST_CONVERSION_PLAN.md` §8); each is flagged at its site.

// See the note in `geometry.rs`: index loops are deliberate here so the port
// can be read against the original line by line.
#![allow(clippy::needless_range_loop)]

use glam::DVec3;
use std::f64::consts::PI;

use crate::geometry::Geometry;
use crate::input::{EllipsoidInput, Projection};
use crate::rotate::{angle_between_planes, rotate_point, unfold_angle};

/// A pair of points forming one rung of a panel strip: the left edge and the
/// right edge at the same latitude.
pub type Rung = [DVec3; 2];

/// An unrolled flat pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatGeometry {
    /// Flattened rungs indexed `[phi][theta]`.
    ///
    /// Outer length is `phi_divisions` (one per panel strip — not `+1`, since
    /// strips span between adjacent phi lines); inner is `theta_divisions + 1`.
    ///
    /// For [`Projection::Cylindrical`] these have been remapped into the x/y
    /// drawing plane; for [`Projection::Spherical`] they are left as-is.
    pub edges_flat: Vec<Vec<Rung>>,

    /// The same rungs before flattening, still on the 3D surface.
    pub edges: Vec<Vec<Rung>>,

    /// Index of the widest theta row, carried through from [`Geometry`].
    pub widest_row: usize,
}

/// Axis-aligned bounds over every point in a rung grid.
pub fn edge_extents(edges: &[Vec<Rung>]) -> (DVec3, DVec3) {
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for row in edges {
        for rung in row {
            for p in rung {
                min = min.min(*p);
                max = max.max(*p);
            }
        }
    }
    (min, max)
}

/// Remap a flattened cylindrical point into the drawing plane.
fn to_drawing_plane(v: DVec3) -> DVec3 {
    DVec3::new(-v.y, v.z, v.x)
}

/// The rotation about `p1`→`p2` that lays the half-plane through `folded` flat
/// against the half-plane through `fixed`, continuing it past the axis.
///
/// Signed, and in the same convention as [`rotate_point`] — so the result can be
/// handed straight to it. Correct whatever the fold: acute, obtuse, or already
/// straight.
///
/// # Why this sits beside [`angle_between_planes`] rather than replacing it
///
/// It could replace it: every fold in this file was measured against this, and
/// all but three agreed to `1e-15`. Substituting it everywhere keeps all 63
/// snapshots inside their `1e-12` tolerance.
///
/// It is deliberately not substituted. This module's arithmetic is ordered to
/// match the original's, so that a difference in the last bits means a porting
/// mistake rather than a reassociation (see `rotate.rs`). Computing a fold two
/// ways that agree to rounding would give up that property across the whole
/// unrolling to tidy up three call sites. So this is used at exactly the folds
/// the original *guessed* — plan §8.9 — and the original expression everywhere
/// it was already measuring something.
fn flattening_rotation(p1: DVec3, p2: DVec3, folded: DVec3, fixed: DVec3) -> f64 {
    let axis = (p2 - p1).normalize_or_zero();
    let across = |p: DVec3| {
        let offset = p - p1;
        offset - axis * offset.dot(axis)
    };
    // The fixed side continues *past* the axis, so the target is its opposite.
    let (from, to) = (across(folded), -across(fixed));
    if from.length_squared() < 1e-24 || to.length_squared() < 1e-24 {
        return 0.0;
    }
    from.cross(to).dot(axis).atan2(from.dot(to))
}

/// Unroll `geometry` into a flat pattern.
pub fn compute_flat_geometry(geometry: &Geometry, input: &EllipsoidInput) -> FlatGeometry {
    let theta_min = if input.theta_min == -90.0 {
        -89.0 * PI / 180.0
    } else {
        input.theta_min * PI / 180.0
    };

    // FIXED, plan §8.1 and Appendix P. The original read `theta_min` here,
    // where it plainly meant `theta_max`. The value feeds the sign-flip guards
    // in both projections, all of which test `> 0`, so reading `theta_min` gave
    // a negative number and put them on the wrong side whenever
    // `0 < theta_max < 90` — which folded the `h_top` extension *into* the
    // panel instead of out beyond it.
    //
    // Carried faithfully through the port so that a porting mistake could not
    // hide behind an intentional fix, then corrected once parity was green.
    // Output differs from the JavaScript for 11 of the 63 matrix cases.
    let theta_max = if input.theta_max == 90.0 {
        89.0 * PI / 180.0
    } else {
        input.theta_max * PI / 180.0
    };

    // Note 0.0001 here against 0.001 in `compute_geometry`. Also the original's.
    let h_top = if theta_max <= 0.0 && input.h_top == 0.0 {
        0.0001
    } else {
        input.h_top
    };
    let h_bottom = input.h_bottom;

    let phi_divisions = geometry.phi_divisions;
    let theta_divisions = geometry.theta_divisions;
    let widest_row = geometry.widest_row;
    let surface = &geometry.points;

    // ---------------------------------------------------------------------
    // Build the panel-strip rungs. Strip `ip` spans phi lines ip and ip+1,
    // which is why there are phi_divisions strips over phi_divisions+1 lines.
    // ---------------------------------------------------------------------
    let mut edges: Vec<Vec<Rung>> = Vec::with_capacity(phi_divisions);
    for ip in 0..phi_divisions {
        let mut row = Vec::with_capacity(theta_divisions + 1);
        for it in 0..=theta_divisions {
            row.push([surface[ip][it], surface[ip + 1][it]]);
        }
        edges.push(row);
    }

    let max_z = edge_extents(&edges).1.z;
    let mut flat = edges.clone();

    match input.projection {
        Projection::Spherical => {
            for ip in 0..phi_divisions {
                for it in 0..theta_divisions {
                    // At the very top there is no rung above, so aim at the apex.
                    let top_point = if it == theta_divisions - 1 {
                        DVec3::new(0.0, 0.0, max_z)
                    } else {
                        flat[ip][it + 2][1]
                    };

                    // FIXED, plan §8.9. The two folds that touch the added top
                    // ring were guessed rather than measured — negated at one,
                    // hardcoded to a right angle at the other. Both guesses hold
                    // for a ring that is neither scaled nor shifted, and only
                    // then. `flattening_rotation` computes what they assumed.
                    let mut angle = if h_top > 0.0
                        && ((it == theta_divisions - 2 && theta_max > 0.0)
                            || it == theta_divisions - 1)
                    {
                        flattening_rotation(
                            flat[ip][it + 1][1],
                            flat[ip][it + 1][0],
                            flat[ip][it][0],
                            top_point,
                        )
                    } else {
                        angle_between_planes(
                            flat[ip][it + 1][0],
                            flat[ip][it + 1][1],
                            flat[ip][it][0],
                            top_point,
                        )
                    };

                    // Not a guess: the measurements agree with this one to
                    // within rounding, on every case that has a bottom ring.
                    if h_bottom > 0.0 && it == 0 && theta_min < 0.0 {
                        angle = -angle;
                    }

                    // Rotate this rung and everything below it about rung it+1.
                    // The axis lives at it+1, outside the mutated range, so it
                    // is untouched by this pass.
                    for itr in 0..=it {
                        let p1 = flat[ip][it + 1][1];
                        let p2 = flat[ip][it + 1][0];
                        let v0 = flat[ip][itr][0];
                        flat[ip][itr][0] = rotate_point(p1, p2, v0, angle);
                        let v1 = flat[ip][itr][1];
                        flat[ip][itr][1] = rotate_point(p1, p2, v1, angle);
                    }
                }
            }
        }

        Projection::Cylindrical => {
            for ip in 0..phi_divisions {
                // Unroll downward from the widest row.
                //
                // Note the axis and angle come from `edges`, the untouched 3D
                // geometry — unlike the spherical branch, which reads back from
                // the partially-flattened `flat`.
                for it in 0..widest_row {
                    let top_point = if it == theta_divisions - 1 {
                        DVec3::new(0.0, 0.0, max_z)
                    } else {
                        edges[ip][it + 2][1]
                    };

                    let mut angle = angle_between_planes(
                        edges[ip][it + 1][0],
                        edges[ip][it + 1][1],
                        edges[ip][it][0],
                        top_point,
                    );

                    // Dead in practice: `it` stops below widest_row, which is at
                    // most theta_divisions-1. Kept for fidelity; the spherical
                    // branch tests `theta_divisions - 2` here instead, and that
                    // asymmetry is unexplained (plan §8.6).
                    if h_top > 0.0 && it == theta_divisions && theta_max > 0.0 {
                        angle = -angle;
                    }
                    if h_bottom > 0.0 && it == 0 && theta_min < 0.0 {
                        angle = -angle;
                    }

                    for itr in 0..=it {
                        let p1 = edges[ip][it + 1][1];
                        let p2 = edges[ip][it + 1][0];
                        let v0 = flat[ip][itr][0];
                        flat[ip][itr][0] = rotate_point(p1, p2, v0, angle);
                        let v1 = flat[ip][itr][1];
                        flat[ip][itr][1] = rotate_point(p1, p2, v1, angle);
                    }
                }

                // Unroll upward from the widest row.
                let mut it = theta_divisions;
                while it > widest_row + 1 {
                    let bottom_point = edges[ip][it - 2][0];

                    // FIXED, plan §8.9 — the same guessed fold as in the
                    // spherical branch, at the one rung that carries the added
                    // top ring. Only one here, not two: this loop folds the rung
                    // *above* the axis, so only `it == theta_divisions` involves
                    // the extension at all.
                    let mut angle = if h_top > 0.0 && it == theta_divisions && theta_max > 0.0 {
                        flattening_rotation(
                            edges[ip][it - 1][0],
                            edges[ip][it - 1][1],
                            edges[ip][it][0],
                            bottom_point,
                        )
                    } else {
                        angle_between_planes(
                            edges[ip][it - 1][0],
                            edges[ip][it - 1][1],
                            bottom_point,
                            edges[ip][it][0],
                        )
                    };

                    // Dead in practice: the loop guard keeps it > widest_row+1 >= 1.
                    if h_bottom > 0.0 && it == 0 && theta_min < 0.0 {
                        angle = -angle;
                    }

                    for itr in (it..=theta_divisions).rev() {
                        let p1 = edges[ip][it - 1][0];
                        let p2 = edges[ip][it - 1][1];
                        let v0 = flat[ip][itr][0];
                        flat[ip][itr][0] = rotate_point(p1, p2, v0, angle);
                        let v1 = flat[ip][itr][1];
                        flat[ip][itr][1] = rotate_point(p1, p2, v1, angle);
                    }

                    it -= 1;
                }
            }

            // Unwrap the cylinder: swing each strip, and every strip after it,
            // into a common plane.
            for ip in 0..phi_divisions {
                let prev = if ip == 0 { phi_divisions - 1 } else { ip - 1 };

                // FIXED, plan §8.8. The original used the acute angle between
                // the two planes, which is the unfold angle only while the
                // strips meet obtusely — true from five strips up, false at
                // three and four, where each strip was left short of the page
                // by the supplement of its fold. See [`unfold_angle`].
                //
                // Identical to the original for five strips or more: there the
                // acute angle *is* the unfold angle, and it is returned by the
                // same expression, so nothing moves by even one bit.
                let mut angle = unfold_angle(
                    flat[ip][widest_row][0],
                    flat[ip][widest_row + 1][0],
                    flat[ip][widest_row][1],
                    flat[prev][widest_row][0],
                );

                // phi starts at 0, so the first strip only needs half the swing
                // to land the pattern against the minimum-x plane.
                if ip == 0 {
                    angle /= 2.0;
                }

                for ipr in ip..phi_divisions {
                    for it in 0..=theta_divisions {
                        // The axis is re-read on every single call because it
                        // is itself inside the rotated range: when ipr == ip and
                        // it reaches widest_row / widest_row+1, these very points
                        // get overwritten. Rotating a point about an axis through
                        // it is a no-op mathematically, but hoisting the reads
                        // would still change the last bits. Keep them here.
                        let p1 = flat[ip][widest_row + 1][0];
                        let p2 = flat[ip][widest_row][0];
                        let v0 = flat[ipr][it][0];
                        flat[ipr][it][0] = rotate_point(p1, p2, v0, angle);

                        let p1 = flat[ip][widest_row + 1][0];
                        let p2 = flat[ip][widest_row][0];
                        let v1 = flat[ipr][it][1];
                        flat[ipr][it][1] = rotate_point(p1, p2, v1, angle);
                    }
                }
            }
        }
    }

    // Cylindrical output is unrolled in a different orientation than it is
    // drawn, so remap it into the x/y plane. Spherical is already there.
    let edges_flat = match input.projection {
        Projection::Spherical => flat,
        Projection::Cylindrical => flat
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|rung| [to_drawing_plane(rung[0]), to_drawing_plane(rung[1])])
                    .collect()
            })
            .collect(),
    };

    FlatGeometry {
        edges_flat,
        edges,
        widest_row,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::compute_geometry;
    use crate::rotate::distance;

    fn run(input: &EllipsoidInput) -> (Geometry, FlatGeometry) {
        let g = compute_geometry(input);
        let f = compute_flat_geometry(&g, input);
        (g, f)
    }

    /// Handing [`flattening_rotation`] to [`rotate_point`] lays the fold flat —
    /// whatever the fold. Plan §8.9.
    #[test]
    fn a_fold_rotated_by_this_ends_up_flat() {
        // Hinge along x, with the fixed side running out along +y.
        let p1 = DVec3::new(-1.0, 0.0, 0.0);
        let p2 = DVec3::new(2.0, 0.0, 0.0);
        let fixed = DVec3::new(0.5, 1.5, 0.0);

        for degrees in [5.0, 45.0, 89.0, 90.0, 91.0, 135.0, 175.0, -45.0, -135.0] {
            let fold = degrees * PI / 180.0;
            let folded = DVec3::new(0.5, fold.cos(), fold.sin());

            let angle = flattening_rotation(p1, p2, folded, fixed);
            let landed = rotate_point(p1, p2, folded, angle);

            // In the plane of hinge and fixed side, which here is z = 0.
            assert!(landed.z.abs() < 1e-12, "{degrees}°: z {}", landed.z);
            // On the far side of the hinge from the fixed point, so the strip
            // continues rather than folding back over itself.
            assert!(landed.y < 0.0, "{degrees}°: y {}", landed.y);
            // And rigidly: a rotation cannot move a point off its own circle.
            assert!(
                (distance(p1, landed) - distance(p1, folded)).abs() < 1e-12,
                "{degrees}°"
            );
        }
    }

    /// Why the original's `PI / 2.0` went unnoticed: for a top ring that is
    /// neither scaled nor shifted, it is the right answer.
    #[test]
    fn an_unshaped_top_ring_really_does_fold_through_a_right_angle() {
        let input = EllipsoidInput {
            h_top: 1.5,
            h_top_fraction: 1.0,
            h_top_shift: 0.0,
            theta_max: 45.0,
            projection: Projection::Spherical,
            ..EllipsoidInput::default()
        };
        let (g, _) = run(&input);
        let td = g.theta_divisions;
        let rung = |it: usize| [g.points[0][it], g.points[1][it]];

        let angle = flattening_rotation(rung(td)[1], rung(td)[0], rung(td - 1)[0], {
            // The apex, as the unroll uses at the last rung.
            let max_z = g
                .points
                .iter()
                .flatten()
                .fold(f64::NEG_INFINITY, |z, p| z.max(p.z));
            DVec3::new(0.0, 0.0, max_z)
        });
        assert!(
            (angle.abs() - PI / 2.0).abs() < 1e-9,
            "unshaped fold came out {angle}"
        );

        // Shape the ring and it stops being a right angle, which is the defect.
        let shaped = EllipsoidInput {
            h_top_fraction: 0.5,
            h_top_shift: 0.75,
            ..input
        };
        let (g, _) = run(&shaped);
        let rung = |it: usize| [g.points[0][it], g.points[1][it]];
        let max_z = g
            .points
            .iter()
            .flatten()
            .fold(f64::NEG_INFINITY, |z, p| z.max(p.z));
        let angle = flattening_rotation(
            rung(td)[1],
            rung(td)[0],
            rung(td - 1)[0],
            DVec3::new(0.0, 0.0, max_z),
        );
        assert!(
            (angle.abs() - PI / 2.0).abs() > 0.1,
            "shaped fold came out {angle}, suspiciously close to a right angle"
        );
    }

    #[test]
    fn strip_count_is_one_less_than_the_phi_line_count() {
        let (g, f) = run(&EllipsoidInput::default());
        assert_eq!(f.edges_flat.len(), g.phi_divisions);
        assert_eq!(g.points.len(), g.phi_divisions + 1);
        for row in &f.edges_flat {
            assert_eq!(row.len(), g.theta_divisions + 1);
        }
    }

    #[test]
    fn output_is_finite_for_both_projections() {
        for projection in Projection::ALL {
            let input = EllipsoidInput {
                projection,
                ..Default::default()
            };
            let (_, f) = run(&input);
            assert!(
                f.edges_flat
                    .iter()
                    .flatten()
                    .flatten()
                    .all(|p| p.is_finite()),
                "{projection} produced non-finite points"
            );
        }
    }

    /// Unrolling a developable strip must preserve every edge length.
    ///
    /// This is the invariant the golden files cannot check on their own: a
    /// sign error in a rotation reproduces exactly if it is also present in the
    /// reference, but it cannot survive this. Worth keeping even though the
    /// original had no equivalent.
    #[test]
    fn flattening_preserves_rung_lengths() {
        for projection in Projection::ALL {
            let input = EllipsoidInput {
                projection,
                ..Default::default()
            };
            let (_, f) = run(&input);

            for (ip, (flat_row, edge_row)) in f.edges_flat.iter().zip(f.edges.iter()).enumerate() {
                for (it, (flat_rung, edge_rung)) in flat_row.iter().zip(edge_row.iter()).enumerate()
                {
                    let before = distance(edge_rung[0], edge_rung[1]);
                    let after = distance(flat_rung[0], flat_rung[1]);
                    assert!(
                        (before - after).abs() < 1e-9,
                        "{projection} strip {ip} rung {it}: {before} -> {after}"
                    );
                }
            }
        }
    }

    /// Distances *along* a strip must survive too — this catches errors in the
    /// rung-to-rung rotation angles rather than within a rung.
    #[test]
    fn flattening_preserves_distance_along_a_strip() {
        for projection in Projection::ALL {
            let input = EllipsoidInput {
                projection,
                ..Default::default()
            };
            let (g, f) = run(&input);

            for ip in 0..g.phi_divisions {
                for it in 0..g.theta_divisions {
                    for side in 0..2 {
                        let before = distance(f.edges[ip][it][side], f.edges[ip][it + 1][side]);
                        let after =
                            distance(f.edges_flat[ip][it][side], f.edges_flat[ip][it + 1][side]);
                        assert!(
                            (before - after).abs() < 1e-9,
                            "{projection} strip {ip} rung {it} side {side}: {before} -> {after}"
                        );
                    }
                }
            }
        }
    }
}
