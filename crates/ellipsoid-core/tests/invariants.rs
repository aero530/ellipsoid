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

/// Unrolling a developable strip preserves lengths, whatever the fold.
#[test]
fn flattening_is_isometric_across_the_matrix() {
    let mut failures = Vec::new();

    for (name, input) in cases() {
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
