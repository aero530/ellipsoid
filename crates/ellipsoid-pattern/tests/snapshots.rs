//! SVG snapshots across the parameter matrix.
//!
//! This replaces a parity harness that compared `draw_edges` against a
//! recording of the original `drawEdges` through a paper.js mock. That
//! reference was retired with the JavaScript in Phase 9, and the SVG is a
//! better thing to pin anyway: it is the artefact that actually ships, and a
//! diff in it is readable by a person.
//!
//! Regenerate deliberately, after reading the diff — never to turn a red test
//! green:
//!
//! ```text
//! UPDATE_GOLDEN=1 cargo test -p ellipsoid-pattern
//! ```
//!
//! Cases come from `golden/`, the same matrix `ellipsoid-core` snapshots.

use std::fs;
use std::path::{Path, PathBuf};

use ellipsoid_core::{EllipsoidInput, Projection, Unit, compute_flat_geometry, compute_geometry};
use ellipsoid_pattern::{SvgOptions, build_scene, to_svg};
use serde::Deserialize;

/// The case definition, in the original's field names.
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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn snapshot_dir() -> PathBuf {
    repo_root().join("golden").join("svg")
}

/// Every case in the matrix, without cutouts.
///
/// Layout is what these snapshots are for. Cutouts are covered thoroughly by
/// the unit tests in `cutouts`, and putting them in every case here would mean
/// 63 boolean subtractions on geometry that includes degenerate apexes — slow,
/// and it would make a layout diff hard to read.
fn cases() -> Vec<(String, EllipsoidInput)> {
    let dir = repo_root().join("golden");
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
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
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
            (name, input)
        })
        .collect()
}

fn render(input: &EllipsoidInput) -> String {
    let geometry = compute_geometry(input);
    let flat = compute_flat_geometry(&geometry, input);
    let scene = build_scene(input, &geometry, &flat);
    to_svg(
        &scene,
        &SvgOptions {
            inkscape_layers: input.inkscape_layers,
        },
    )
}

#[test]
fn svg_matches_the_snapshots() {
    let cases = cases();
    assert!(cases.len() >= 50, "expected the full matrix");

    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    if update {
        fs::create_dir_all(snapshot_dir()).expect("create snapshot dir");
    }

    let mut failures = Vec::new();

    for (name, input) in &cases {
        let got = render(input);
        let path = snapshot_dir().join(format!("{name}.svg"));

        if update {
            fs::write(&path, &got).unwrap_or_else(|e| panic!("writing {path:?}: {e}"));
            continue;
        }

        let Ok(want) = fs::read_to_string(&path) else {
            failures.push(format!(
                "  {name}: no snapshot at {} — regenerate with UPDATE_GOLDEN=1",
                path.display()
            ));
            continue;
        };

        if got != want {
            // Report the first differing line: whole-SVG diffs are unreadable
            // in test output, and the line number is enough to find it.
            let at = got
                .lines()
                .zip(want.lines())
                .position(|(a, b)| a != b)
                .map(|i| i + 1);
            failures.push(match at {
                Some(line) => format!("  {name}: differs at line {line}"),
                None => format!(
                    "  {name}: same prefix, different length ({} vs {} lines)",
                    got.lines().count(),
                    want.lines().count()
                ),
            });
        }
    }

    if update {
        eprintln!("regenerated {} SVG snapshots", cases.len());
        return;
    }

    assert!(
        failures.is_empty(),
        "{} of {} SVG snapshots changed:\n{}\n\nIf intended, review the diff and \
         regenerate with UPDATE_GOLDEN=1.",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

/// The gap filter is what merges adjacent panels into a single cut, so a larger
/// threshold must drop outline points. An invariant rather than a snapshot: it
/// says what `min_gap` is *for*.
#[test]
fn min_gap_removes_outline_points() {
    use ellipsoid_pattern::{Item, LAYER_PATTERN, draw_edges};

    let count = |min_gap: f64| {
        let input = EllipsoidInput {
            min_gap,
            ..Default::default()
        };
        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);
        let scene = draw_edges(&input, &flat);
        match &scene.layer(LAYER_PATTERN).expect("pattern layer").items[0] {
            Item::Path { points, .. } => points.len(),
            other => panic!("expected the outline path, got {other:?}"),
        }
    };

    assert!(
        count(0.05) < count(0.001),
        "a larger min_gap should drop outline points: {} vs {}",
        count(0.05),
        count(0.001)
    );
}

#[test]
fn every_expected_layer_is_present() {
    use ellipsoid_pattern::{
        LAYER_BOUNDING_BOX, LAYER_GUIDE_LINES, LAYER_PATTERN, LAYER_QUADS_DEST, LAYER_QUADS_SRC,
        draw_edges,
    };

    let input = EllipsoidInput::default();
    let geometry = compute_geometry(&input);
    let flat = compute_flat_geometry(&geometry, &input);
    let scene = draw_edges(&input, &flat);

    for name in [
        LAYER_PATTERN,
        LAYER_BOUNDING_BOX,
        LAYER_GUIDE_LINES,
        LAYER_QUADS_DEST,
        LAYER_QUADS_SRC,
    ] {
        assert!(scene.layer(name).is_some(), "missing layer {name}");
    }
}
