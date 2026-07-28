//! Geometry snapshots across the parameter matrix.
//!
//! These fixtures began life as a parity harness: `tools/extract-golden.mjs`
//! ran the original `app/utils/ellipsoid.js` and this test held the port to its
//! output at `1e-9`. That job is done — the port was validated, the JavaScript
//! was retired in Phase 9, and the `theta_max` fix (plan §8.1) means the two
//! now deliberately disagree on 11 of the cases. Holding a defunct
//! implementation up as the source of truth stopped paying for itself.
//!
//! So `golden/` is now generated from **this** implementation and the test is a
//! regression check: any change to the geometry pipeline that moves a number
//! shows up here, on a matrix chosen to hit the branches rather than the
//! defaults. Reviewing a diff is the point — regenerate deliberately, never to
//! make a red test go green:
//!
//! ```text
//! UPDATE_GOLDEN=1 cargo test -p ellipsoid-core
//! ```
//!
//! The `settings` block in each fixture is the case definition. Adding a case
//! means adding a file with one, then regenerating.
//!
//! What this cannot do, and `flattening_is_isometric_across_the_matrix` in
//! `invariants.rs` can, is say whether the numbers are *right*. Snapshots pin
//! behaviour; invariants judge it. Both are needed.

use std::fs;
use std::path::{Path, PathBuf};

use ellipsoid_core::obj::parse_obj;
use ellipsoid_core::{
    EllipsoidInput, Projection, Unit, compute_flat_geometry, compute_geometry, flat_to_obj,
    geometry_to_obj,
};
use serde::{Deserialize, Serialize};

/// Snapshots are this implementation's own output, so they should agree to the
/// bit. The tolerance is here only because a compiler or libm change may move
/// the last place or two, which is not a regression worth failing over.
const TOL: f64 = 1e-12;

fn close(a: f64, b: f64) -> bool {
    let scale = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() <= TOL * scale
}

/// The case definition, in the original's field names.
///
/// Kept as-is so the matrix did not have to be retyped when the fixtures
/// changed hands. Only these fields affect geometry.
#[derive(Debug, Deserialize, Serialize)]
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

impl Settings {
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
            // None of these reach the geometry; the SVG snapshots cover them.
            unit: Unit::Inch,
            image_offset: 0.5,
            min_gap: 0.001,
            inkscape_layers: true,
            cutouts: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Geometry {
    phi_divisions: usize,
    theta_divisions: usize,
    widest_row: usize,
    points: Vec<Vec<[f64; 3]>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Flat {
    widest_row: usize,
    edges_flat: Vec<Vec<[[f64; 3]; 2]>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Obj {
    surface: String,
    flat: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Snapshot {
    name: String,
    settings: Settings,
    geometry: Geometry,
    flat: Flat,
    obj: Obj,
}

fn golden_dir() -> PathBuf {
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
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    files
}

/// Recompute a case from its settings.
fn compute(settings: &Settings, name: &str) -> Snapshot {
    let input = settings.to_input();
    let geometry = compute_geometry(&input);
    let flat = compute_flat_geometry(&geometry, &input);

    Snapshot {
        name: name.to_string(),
        settings: Settings {
            projection: settings.projection.clone(),
            ..*settings
        },
        geometry: Geometry {
            phi_divisions: geometry.phi_divisions,
            theta_divisions: geometry.theta_divisions,
            widest_row: geometry.widest_row,
            points: geometry
                .points
                .iter()
                .map(|row| row.iter().map(|p| [p.x, p.y, p.z]).collect())
                .collect(),
        },
        flat: Flat {
            widest_row: flat.widest_row,
            edges_flat: flat
                .edges_flat
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|r| [[r[0].x, r[0].y, r[0].z], [r[1].x, r[1].y, r[1].z]])
                        .collect()
                })
                .collect(),
        },
        obj: Obj {
            surface: geometry_to_obj(&geometry),
            flat: flat_to_obj(&flat),
        },
    }
}

/// First difference between two snapshots, if any.
fn compare(got: &Snapshot, want: &Snapshot) -> Result<(), String> {
    if got.geometry.phi_divisions != want.geometry.phi_divisions
        || got.geometry.theta_divisions != want.geometry.theta_divisions
        || got.geometry.widest_row != want.geometry.widest_row
        || got.flat.widest_row != want.flat.widest_row
    {
        return Err("division counts or widest row changed".into());
    }

    if got.geometry.points.len() != want.geometry.points.len() {
        return Err(format!(
            "surface rows: got {}, want {}",
            got.geometry.points.len(),
            want.geometry.points.len()
        ));
    }
    for (ip, (g, w)) in got
        .geometry
        .points
        .iter()
        .zip(&want.geometry.points)
        .enumerate()
    {
        for (it, (gp, wp)) in g.iter().zip(w).enumerate() {
            for axis in 0..3 {
                if !close(gp[axis], wp[axis]) {
                    return Err(format!(
                        "surface point [{ip}][{it}] axis {axis}: got {}, want {}",
                        gp[axis], wp[axis]
                    ));
                }
            }
        }
    }

    if got.flat.edges_flat.len() != want.flat.edges_flat.len() {
        return Err("flat strip count changed".into());
    }
    for (ip, (g, w)) in got
        .flat
        .edges_flat
        .iter()
        .zip(&want.flat.edges_flat)
        .enumerate()
    {
        for (it, (gr, wr)) in g.iter().zip(w).enumerate() {
            for side in 0..2 {
                for axis in 0..3 {
                    if !close(gr[side][axis], wr[side][axis]) {
                        return Err(format!(
                            "flat point [{ip}][{it}][{side}] axis {axis}: got {}, want {}",
                            gr[side][axis], wr[side][axis]
                        ));
                    }
                }
            }
        }
    }

    // OBJ is compared as parsed numbers, never as text: the two differ in
    // exponent formatting for values near zero without differing in value.
    for (label, mine, theirs) in [
        ("surface", &got.obj.surface, &want.obj.surface),
        ("flat", &got.obj.flat, &want.obj.flat),
    ] {
        let (gv, gf) = parse_obj(mine);
        let (wv, wf) = parse_obj(theirs);
        if gf != wf {
            return Err(format!("{label} OBJ face topology changed"));
        }
        if gv.len() != wv.len() {
            return Err(format!(
                "{label} OBJ vertex count: got {}, want {}",
                gv.len(),
                wv.len()
            ));
        }
        for (i, (g, w)) in gv.iter().zip(&wv).enumerate() {
            for axis in 0..3 {
                if !close(g[axis], w[axis]) {
                    return Err(format!(
                        "{label} OBJ vertex {i} axis {axis}: got {}, want {}",
                        g[axis], w[axis]
                    ));
                }
            }
        }
    }

    Ok(())
}

#[test]
fn geometry_matches_the_snapshots() {
    let files = golden_files();
    assert!(
        files.len() >= 50,
        "expected the full matrix, found {} files in {}",
        files.len(),
        golden_dir().display()
    );

    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let mut failures = Vec::new();

    for path in &files {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("fixture name");

        // Only the settings are read back when regenerating, so a fixture in
        // any older shape still defines its case.
        #[derive(Deserialize)]
        struct CaseOnly {
            settings: Settings,
        }
        let case: CaseOnly =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing {path:?}: {e}"));
        let got = compute(&case.settings, stem);

        if update {
            let json = serde_json::to_string_pretty(&got).expect("serialise snapshot");
            fs::write(path, json + "\n").unwrap_or_else(|e| panic!("writing {path:?}: {e}"));
            continue;
        }

        match serde_json::from_str::<Snapshot>(&text) {
            Ok(want) => {
                if let Err(why) = compare(&got, &want) {
                    failures.push(format!("  {stem}: {why}"));
                }
            }
            Err(e) => failures.push(format!(
                "  {stem}: cannot read snapshot ({e}) — regenerate with UPDATE_GOLDEN=1"
            )),
        }
    }

    if update {
        eprintln!("regenerated {} snapshots", files.len());
        return;
    }

    assert!(
        failures.is_empty(),
        "{} of {} snapshots changed:\n{}\n\nIf the change is intended, review the \
         diff and regenerate with UPDATE_GOLDEN=1.",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}
