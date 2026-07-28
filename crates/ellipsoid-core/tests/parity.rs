//! Golden-file parity against the original JavaScript implementation.
//!
//! Fixtures under `tests/golden/` are produced by `tools/extract-golden.mjs`,
//! which runs the untouched `app/utils/ellipsoid.js`. This is the gate for
//! Phase 1 of `RUST_CONVERSION_PLAN.md`: the Rust port must reproduce the
//! reference across a matrix chosen to hit its branches, not just its defaults.
//!
//! Regenerate with `node tools/extract-golden.mjs`.

use std::fs;
use std::path::{Path, PathBuf};

use ellipsoid_core::obj::parse_obj;
use ellipsoid_core::{
    EllipsoidInput, Projection, Unit, compute_flat_geometry, compute_geometry, flat_to_obj,
    geometry_to_obj,
};
use glam::DVec3;
use serde::Deserialize;

/// Coordinates are O(1)–O(10) drawing units, so compare with a relative
/// tolerance that degrades to absolute near zero. Pure relative comparison
/// would be meaningless for values like `-2.9e-16` that should be zero.
///
/// This is far looser than one ULP (~2.2e-16) on purpose: V8's `Math.cos` and
/// the platform libm need not agree to the last bit, and those differences
/// compound through the rotation chain. It is still tight enough that any real
/// porting error shows up immediately.
const TOL: f64 = 1e-9;

fn close(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let scale = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() <= TOL * scale
}

#[derive(Debug, Deserialize)]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}

impl Point {
    fn matches(&self, v: DVec3) -> bool {
        close(self.x, v.x) && close(self.y, v.y) && close(self.z, v.z)
    }

    fn delta(&self, v: DVec3) -> f64 {
        (self.x - v.x)
            .abs()
            .max((self.y - v.y).abs())
            .max((self.z - v.z).abs())
    }
}

/// The original's settings object, field-for-field.
#[derive(Debug, Deserialize)]
struct JsSettings {
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

impl JsSettings {
    fn to_input(&self) -> EllipsoidInput {
        EllipsoidInput {
            a: self.a,
            b: self.b,
            c: self.c,
            h_top: self.h_top,
            h_middle: self.h_middle,
            h_bottom: self.h_bottom,
            h_top_fraction: self.h_top_fraction,
            h_top_shift: self.h_top_shift,
            phi_divisions: self.phi_divisions,
            theta_divisions: self.divisions,
            theta_min: self.theta_min,
            theta_max: self.theta_max,
            projection: match self.projection.as_str() {
                "spherical" => Projection::Spherical,
                "cylindrical" => Projection::Cylindrical,
                other => panic!("unknown projection {other:?}"),
            },
            // None of these affect geometry; they matter from Phase 2 onward.
            unit: Unit::Inch,
            image_offset: 0.5,
            min_gap: 0.001,
            inkscape_layers: true,
            // The reference predates cutouts, so parity always runs without them.
            cutouts: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GoldenGeometry {
    #[serde(rename = "Divisions")]
    phi_divisions: usize,
    divisions: usize,
    #[serde(rename = "indexWide")]
    index_wide: usize,
    points: Vec<Vec<Point>>,
    obj: String,
}

#[derive(Debug, Deserialize)]
struct GoldenFlat {
    #[serde(rename = "indexWide")]
    index_wide: usize,
    #[serde(rename = "edgesFlat")]
    edges_flat: Vec<Vec<Vec<Point>>>,
    obj: String,
}

#[derive(Debug, Deserialize)]
struct Golden {
    name: String,
    settings: JsSettings,
    geometry: GoldenGeometry,
    flat: GoldenFlat,
}

fn golden_dir() -> PathBuf {
    // Workspace-root `golden/`, shared with ellipsoid-pattern.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("golden")
}

fn golden_files() -> Vec<PathBuf> {
    let dir = golden_dir();
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name().is_some_and(|n| n != "index.json")
        })
        .collect();
    files.sort();
    files
}

/// Check one case, returning a description of the first divergence found.
fn check(golden: &Golden) -> Result<(), String> {
    let input = golden.settings.to_input();
    let geometry = compute_geometry(&input);

    if geometry.phi_divisions != golden.geometry.phi_divisions {
        return Err(format!(
            "phi_divisions: got {}, want {}",
            geometry.phi_divisions, golden.geometry.phi_divisions
        ));
    }
    if geometry.theta_divisions != golden.geometry.divisions {
        return Err(format!(
            "theta_divisions: got {}, want {}",
            geometry.theta_divisions, golden.geometry.divisions
        ));
    }
    if geometry.widest_row != golden.geometry.index_wide {
        return Err(format!(
            "widest_row: got {}, want {}",
            geometry.widest_row, golden.geometry.index_wide
        ));
    }

    if geometry.points.len() != golden.geometry.points.len() {
        return Err(format!(
            "surface phi rows: got {}, want {}",
            geometry.points.len(),
            golden.geometry.points.len()
        ));
    }
    for (ip, (got_row, want_row)) in geometry
        .points
        .iter()
        .zip(golden.geometry.points.iter())
        .enumerate()
    {
        if got_row.len() != want_row.len() {
            return Err(format!(
                "surface row {ip}: got {} points, want {}",
                got_row.len(),
                want_row.len()
            ));
        }
        for (it, (got, want)) in got_row.iter().zip(want_row.iter()).enumerate() {
            if !want.matches(*got) {
                return Err(format!(
                    "surface point [{ip}][{it}]: got {got:?}, want ({}, {}, {}), delta {:.3e}",
                    want.x,
                    want.y,
                    want.z,
                    want.delta(*got)
                ));
            }
        }
    }

    let flat = compute_flat_geometry(&geometry, &input);

    if flat.widest_row != golden.flat.index_wide {
        return Err(format!(
            "flat widest_row: got {}, want {}",
            flat.widest_row, golden.flat.index_wide
        ));
    }
    if flat.edges_flat.len() != golden.flat.edges_flat.len() {
        return Err(format!(
            "flat strips: got {}, want {}",
            flat.edges_flat.len(),
            golden.flat.edges_flat.len()
        ));
    }
    for (ip, (got_row, want_row)) in flat
        .edges_flat
        .iter()
        .zip(golden.flat.edges_flat.iter())
        .enumerate()
    {
        if got_row.len() != want_row.len() {
            return Err(format!(
                "flat strip {ip}: got {} rungs, want {}",
                got_row.len(),
                want_row.len()
            ));
        }
        for (it, (got, want)) in got_row.iter().zip(want_row.iter()).enumerate() {
            for side in 0..2 {
                if !want[side].matches(got[side]) {
                    return Err(format!(
                        "flat point [{ip}][{it}][{side}]: got {:?}, want ({}, {}, {}), delta {:.3e}",
                        got[side],
                        want[side].x,
                        want[side].y,
                        want[side].z,
                        want[side].delta(got[side])
                    ));
                }
            }
        }
    }

    Ok(())
}

#[test]
fn matches_javascript_reference() {
    let files = golden_files();

    // Guard against a silently empty fixture directory passing as success.
    assert!(
        files.len() >= 50,
        "expected the full golden matrix, found {} files in {} — \
         regenerate with `node tools/extract-golden.mjs`",
        files.len(),
        golden_dir().display()
    );

    let mut failures = Vec::new();

    for path in &files {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
        let golden: Golden =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing {path:?}: {e}"));

        if let Err(why) = check(&golden) {
            failures.push(format!("  {}: {why}", golden.name));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} golden cases diverged from the JavaScript reference:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );

    eprintln!("{} golden cases matched", files.len());
}

/// OBJ meshes must match the reference in both vertex positions and face
/// topology.
///
/// Compared as parsed numbers, never as text: JavaScript switches to exponent
/// notation below `1e-6` and Rust does not, so `-2.88e-16` and
/// `-0.000000000000000288` are the same value written two ways.
#[test]
fn obj_output_matches_javascript_reference() {
    let mut failures = Vec::new();

    for path in golden_files() {
        let text = fs::read_to_string(&path).expect("read golden");
        let golden: Golden = serde_json::from_str(&text).expect("parse golden");
        let input = golden.settings.to_input();

        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);

        let cases = [
            ("surface", geometry_to_obj(&geometry), &golden.geometry.obj),
            ("flat", flat_to_obj(&flat), &golden.flat.obj),
        ];

        for (label, mine, theirs) in cases {
            let (got_v, got_f) = parse_obj(&mine);
            let (want_v, want_f) = parse_obj(theirs);

            if got_v.len() != want_v.len() {
                failures.push(format!(
                    "  {} [{label}]: {} vertices, want {}",
                    golden.name,
                    got_v.len(),
                    want_v.len()
                ));
                continue;
            }
            // Face indices are pure integer arithmetic — they must match exactly.
            if got_f != want_f {
                let first = got_f
                    .iter()
                    .zip(want_f.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(0);
                failures.push(format!(
                    "  {} [{label}]: face {first} is {:?}, want {:?} ({} vs {} faces)",
                    golden.name,
                    got_f.get(first),
                    want_f.get(first),
                    got_f.len(),
                    want_f.len()
                ));
                continue;
            }

            for (i, (got, want)) in got_v.iter().zip(want_v.iter()).enumerate() {
                for axis in 0..3 {
                    if !close(got[axis], want[axis]) {
                        failures.push(format!(
                            "  {} [{label}]: vertex {i} axis {axis}: got {}, want {}",
                            golden.name, got[axis], want[axis]
                        ));
                        break;
                    }
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} OBJ divergences:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The isometry invariant, checked across the whole matrix rather than just the
/// defaults covered by the unit tests in `flatten`.
///
/// Golden files cannot catch a rotation-sign error that is also present in the
/// reference. This can: unrolling a developable strip has to preserve lengths
/// no matter what the reference did.
#[test]
fn flattening_is_isometric_across_the_matrix() {
    let mut failures = Vec::new();

    for path in golden_files() {
        let text = fs::read_to_string(&path).expect("read golden");
        let golden: Golden = serde_json::from_str(&text).expect("parse golden");
        let input = golden.settings.to_input();

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
            failures.push(format!("  {}: worst length error {worst:.3e}", golden.name));
        }
    }

    assert!(
        failures.is_empty(),
        "{} cases were not isometric:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
