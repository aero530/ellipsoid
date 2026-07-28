//! Properties the geometry must have, checked across the parameter matrix.
//!
//! `snapshots.rs` pins what the code currently produces; this says whether that
//! is *right*. The distinction earns its keep twice over here. It caught a
//! rotation-sign error that the old parity harness could not, because the error
//! was present in the reference too — and it is what made the `theta_max` fix
//! (plan §8.1) safe to reason about, since it holds for both the old fold and
//! the new one. Two valid unrollings, folded differently.

use std::fs;
use std::path::{Path, PathBuf};

use ellipsoid_core::{EllipsoidInput, Projection, Unit, compute_flat_geometry, compute_geometry};
use serde::Deserialize;

/// The case definition, in the original's field names. See `snapshots.rs`.
#[derive(Debug, Deserialize)]
struct Settings {
    a: f64,
    b: f64,
    c: f64,
    #[serde(rename = "hTop")]
    h_top: f64,
    #[serde(rename = "hMiddle")]
    h_middle: f64,
    #[serde(rename = "hBottom")]
    h_bottom: f64,
    #[serde(rename = "hTopFraction")]
    h_top_fraction: f64,
    #[serde(rename = "hTopShift")]
    h_top_shift: f64,
    #[serde(rename = "Divisions")]
    phi_divisions: usize,
    divisions: usize,
    #[serde(rename = "thetaMin")]
    theta_min: f64,
    #[serde(rename = "thetaMax")]
    theta_max: f64,
    projection: String,
}

#[derive(Debug, Deserialize)]
struct Case {
    settings: Settings,
}

fn cases() -> Vec<(String, EllipsoidInput)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("golden");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("read case");
            let case: Case = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
            let s = case.settings;
            let input = EllipsoidInput {
                a: s.a,
                b: s.b,
                c: s.c,
                h_top: s.h_top,
                h_middle: s.h_middle,
                h_bottom: s.h_bottom,
                h_top_fraction: s.h_top_fraction,
                h_top_shift: s.h_top_shift,
                phi_divisions: s.phi_divisions,
                theta_divisions: s.divisions,
                theta_min: s.theta_min,
                theta_max: s.theta_max,
                projection: match s.projection.as_str() {
                    "spherical" => Projection::Spherical,
                    "cylindrical" => Projection::Cylindrical,
                    other => panic!("unknown projection {other:?}"),
                },
                unit: Unit::Inch,
                image_offset: 0.5,
                min_gap: 0.001,
                inkscape_layers: true,
                cutouts: Vec::new(),
            };
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            (name, input)
        })
        .collect()
}

/// The golden matrix, plus a dense sweep over the division counts.
///
/// The matrix samples divisions at three points, which was enough to miss §8.8:
/// the cylindrical unwrap was wrong at three and four strips and right from five
/// up, and only one case in the matrix used three. Sweeping them is cheap.
fn cases_and_division_sweep() -> Vec<(String, EllipsoidInput)> {
    let mut all = cases();
    for projection in [Projection::Spherical, Projection::Cylindrical] {
        for phi in 3..=13usize {
            for theta in [3usize, 4, 7, 16] {
                all.push((
                    format!("sweep_{projection:?}_phi{phi}_theta{theta}").to_lowercase(),
                    EllipsoidInput {
                        a: 3.75,
                        b: 2.875,
                        c: 3.0,
                        h_top: 1.0,
                        h_middle: 2.0,
                        h_bottom: 2.0,
                        h_top_fraction: 0.75,
                        h_top_shift: 0.125,
                        phi_divisions: phi,
                        theta_divisions: theta,
                        theta_min: -35.0,
                        theta_max: 90.0,
                        projection,
                        unit: Unit::Inch,
                        image_offset: 0.5,
                        min_gap: 0.001,
                        inkscape_layers: true,
                        cutouts: Vec::new(),
                    },
                ));
            }
        }
    }
    all
}

/// How far out of the drawing plane a case lands, and how much length that
/// costs the drawing.
///
/// `edges_flat` is drawn as `(x, y)` with the third axis discarded, so a pattern
/// that does not share one `z` is drawn as the *projection* of something tilted —
/// foreshortened, and cut short if you believe it.
fn out_of_plane(input: &EllipsoidInput) -> (f64, f64) {
    let geometry = compute_geometry(input);
    let flat = compute_flat_geometry(&geometry, input);

    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut foreshortened = 0.0f64;
    for row in &flat.edges_flat {
        for rung in row {
            for p in rung {
                lo = lo.min(p.z);
                hi = hi.max(p.z);
            }
        }
        for (rung, above) in row.iter().zip(row.iter().skip(1)) {
            for (a, b) in rung.iter().zip(above.iter()) {
                let drawn = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
                foreshortened = foreshortened.max(a.distance(*b) - drawn);
            }
        }
    }
    (hi - lo, foreshortened)
}

/// Nothing may be drawn shorter than it is.
///
/// The companion to [`unrolling_lands_in_one_plane`], and the more direct
/// statement of what being out of plane costs: a tilted extension was drawn as
/// its own projection, up to 36% short on the edge concerned. Plan §8.9.
#[test]
fn nothing_is_drawn_shorter_than_it_is() {
    let mut failures = Vec::new();

    for (name, input) in cases_and_division_sweep() {
        let (_, short) = out_of_plane(&input);
        if short > 1e-9 {
            failures.push(format!("  {name}: drawn {short:.3e} short"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} cases were foreshortened:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// A flat pattern is flat.
///
/// A fold left short of straight leaves the strips after it standing out of the
/// page, and the drawing silently projects them away: at three strips one panel
/// came out edge-on, of zero width, and the next folded back over the first. See
/// the plan's §8.8.
///
/// The isometry check below cannot see this — swinging a strip to the wrong angle
/// is still a rigid motion, so every length survives it.
///
/// This held for everything except a *shaped* top extension until §8.9 was
/// fixed; it now covers the whole matrix, extensions included.
#[test]
fn unrolling_lands_in_one_plane() {
    let mut failures = Vec::new();

    for (name, input) in cases_and_division_sweep() {
        let (spread, _) = out_of_plane(&input);
        if spread > 1e-9 {
            failures.push(format!("  {name}: out of plane by {spread:.3e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} cases did not unroll into the drawing plane:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Cylindrical gores are laid out in order, joined along the widest row.
///
/// Unrolling a cylinder puts the strips side by side, each starting where the
/// last one ended. The seam is shared exactly, and going left to right the
/// strips follow their phi order — a strip swung too little, or too far, breaks
/// one or both.
///
/// Spherical is excluded deliberately: it unrolls every petal about the pole,
/// leaving them stacked, and `ellipsoid-pattern`'s layout fans them out when it
/// draws. Only the cylindrical projection lays them out here.
#[test]
fn cylindrical_gores_are_laid_side_by_side() {
    let mut failures = Vec::new();

    for (name, input) in cases_and_division_sweep() {
        if input.projection != Projection::Cylindrical {
            continue;
        }
        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);
        let widest = flat.widest_row;

        for ip in 0..input.phi_divisions.saturating_sub(1) {
            let [left, right] = flat.edges_flat[ip][widest];
            let next = flat.edges_flat[ip + 1][widest];

            let gap = right.distance(next[0]);
            if gap > 1e-9 {
                failures.push(format!(
                    "  {name}: strip {ip} and {} are {gap:.3e} apart at the seam",
                    ip + 1
                ));
            }
            if !(next[1].x > right.x && right.x > left.x) {
                failures.push(format!(
                    "  {name}: strip {} does not follow strip {ip} along x ({:.4} then {:.4} then {:.4})",
                    ip + 1,
                    left.x,
                    right.x,
                    next[1].x
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} layout faults:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Unrolling a developable strip preserves lengths, whatever the fold.
#[test]
fn flattening_is_isometric_across_the_matrix() {
    let mut failures = Vec::new();

    for (name, input) in cases_and_division_sweep() {
        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);

        let mut worst = 0.0f64;
        for (row_flat, row_edge) in flat.edges_flat.iter().zip(flat.edges.iter()) {
            // Across each rung.
            for (rung_flat, rung_edge) in row_flat.iter().zip(row_edge.iter()) {
                let before = rung_edge[0].distance(rung_edge[1]);
                let after = rung_flat[0].distance(rung_flat[1]);
                worst = worst.max((before - after).abs());
            }
            // And along the strip.
            for it in 0..row_flat.len().saturating_sub(1) {
                for side in 0..2 {
                    let before = row_edge[it][side].distance(row_edge[it + 1][side]);
                    let after = row_flat[it][side].distance(row_flat[it + 1][side]);
                    worst = worst.max((before - after).abs());
                }
            }
        }

        if worst > 1e-9 {
            failures.push(format!("  {name}: worst length error {worst:.3e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} cases were not isometric:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
