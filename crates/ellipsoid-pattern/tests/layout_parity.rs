//! Golden-file parity for the 2D layout, against `drawEdges` in the original
//! `app/utils/ellipsoid.js`.
//!
//! `drawEdges` only writes into the paper.js scope — it never reads geometry
//! back — so `tools/extract-golden.mjs` captures its exact output through a
//! recording mock, with no paper.js, no canvas, and no npm install. Fixtures
//! carrying a `draw` block are the ones to check here.
//!
//! `drawNotes` is deliberately not covered: it reads `bounds` off real paper
//! items, so it cannot be captured the same way. See `RUST_CONVERSION_PLAN.md`
//! §7.2 — its text placement is approximate by design and affects no cut
//! geometry.

use std::fs;
use std::path::{Path, PathBuf};

use ellipsoid_core::{EllipsoidInput, Projection, Unit, compute_flat_geometry, compute_geometry};
use ellipsoid_pattern::{
    Item, LAYER_BOUNDING_BOX, LAYER_GUIDE_LINES, LAYER_PATTERN, LAYER_QUADS_DEST, LAYER_QUADS_SRC,
    draw_edges,
};
use glam::DVec2;
use serde::Deserialize;

/// Full-precision values captured straight from the mock.
const TOL: f64 = 1e-9;

/// The cut outline reaches the golden file as a path string the original had
/// already rounded to three decimals, so half a unit in the last place is the
/// tightest meaningful bound.
const TOL_PATH: f64 = 1e-3;

fn close(a: f64, b: f64, tol: f64) -> bool {
    a == b || (a - b).abs() <= tol * a.abs().max(b.abs()).max(1.0)
}

// ---------------------------------------------------------------------------
// Fixture shapes
// ---------------------------------------------------------------------------

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
    ppu: f64,
    #[serde(rename = "minGap")]
    min_gap: f64,
    #[serde(rename = "imageOffset")]
    image_offset: f64,
}

impl JsSettings {
    fn to_input(&self) -> EllipsoidInput {
        let unit = Unit::ALL
            .into_iter()
            .find(|u| (u.px_per_unit() - self.ppu).abs() < 1e-6)
            .unwrap_or_else(|| panic!("no Unit matches ppu {}", self.ppu));

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
            unit,
            image_offset: self.image_offset,
            min_gap: self.min_gap,
            inkscape_layers: true,
            // The reference predates cutouts, so parity always runs without them.
            cutouts: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GoldenItem {
    kind: String,
    #[serde(default)]
    d: Option<String>,
    #[serde(default)]
    points: Option<Vec<[f64; 2]>>,
    #[serde(default)]
    from: Option<[f64; 2]>,
    #[serde(default)]
    to: Option<[f64; 2]>,
    #[serde(default)]
    children: Option<Vec<GoldenItem>>,
}

#[derive(Debug, Deserialize)]
struct GoldenLayer {
    name: String,
    items: Vec<GoldenItem>,
}

#[derive(Debug, Deserialize)]
struct GoldenDraw {
    layers: Vec<GoldenLayer>,
}

#[derive(Debug, Deserialize)]
struct Golden {
    name: String,
    settings: JsSettings,
    #[serde(default)]
    draw: Option<GoldenDraw>,
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("golden")
}

fn goldens_with_draw() -> Vec<Golden> {
    let dir = golden_dir();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name().is_some_and(|n| n != "index.json")
        })
        .collect();
    paths.sort();

    paths
        .iter()
        .map(|p| {
            let text = fs::read_to_string(p).unwrap_or_else(|e| panic!("reading {p:?}: {e}"));
            serde_json::from_str::<Golden>(&text).unwrap_or_else(|e| panic!("parsing {p:?}: {e}"))
        })
        .filter(|g| g.draw.is_some())
        .collect()
}

/// Parse the `M x,y L x,y ... z` string the original built by hand.
fn parse_path_data(d: &str) -> Vec<DVec2> {
    d.split_whitespace()
        .filter_map(|token| {
            let body = token
                .strip_prefix('M')
                .or_else(|| token.strip_prefix('L'))?;
            let (x, y) = body.split_once(',')?;
            Some(DVec2::new(x.parse().ok()?, y.parse().ok()?))
        })
        .collect()
}

/// Flatten our item tree into comparable point lists, in emission order.
fn item_points(item: &Item, out: &mut Vec<Vec<DVec2>>) {
    match item {
        Item::Path { points, .. } => out.push(points.clone()),
        Item::Rect { bounds, .. } => out.push(vec![bounds.min, bounds.max]),
        Item::Group(children) => {
            for c in children {
                item_points(c, out);
            }
        }
        Item::Text(_) => {}
    }
}

fn golden_item_points(item: &GoldenItem, out: &mut Vec<Vec<DVec2>>) {
    match item.kind.as_str() {
        "path" => {
            if let Some(d) = &item.d {
                out.push(parse_path_data(d));
            } else if let Some(points) = &item.points {
                out.push(points.iter().map(|p| DVec2::new(p[0], p[1])).collect());
            } else {
                out.push(Vec::new());
            }
        }
        "rect" => {
            let (from, to) = (item.from.expect("rect from"), item.to.expect("rect to"));
            out.push(vec![DVec2::new(from[0], from[1]), DVec2::new(to[0], to[1])]);
        }
        "group" => {
            for c in item.children.iter().flatten() {
                golden_item_points(c, out);
            }
        }
        other => panic!("unexpected golden item kind {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn layout_matches_javascript_reference() {
    let goldens = goldens_with_draw();
    assert!(
        goldens.len() >= 12,
        "expected the drawEdges fixtures, found {} — regenerate with \
         `node tools/extract-golden.mjs`",
        goldens.len()
    );

    let mut failures = Vec::new();
    let mut compared = 0usize;

    for golden in &goldens {
        let input = golden.settings.to_input();
        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);
        let scene = draw_edges(&input, &flat);
        let want = golden.draw.as_ref().unwrap();

        let got_names: Vec<&str> = scene.layers.iter().map(|l| l.name.as_str()).collect();
        let want_names: Vec<&str> = want.layers.iter().map(|l| l.name.as_str()).collect();
        if got_names != want_names {
            failures.push(format!(
                "  {}: layers {got_names:?}, want {want_names:?}",
                golden.name
            ));
            continue;
        }

        for (layer, want_layer) in scene.layers.iter().zip(want.layers.iter()) {
            // The cut outline arrives pre-rounded to three decimals.
            let tol = if layer.name == LAYER_PATTERN {
                TOL_PATH
            } else {
                TOL
            };

            let mut got_pts = Vec::new();
            for item in &layer.items {
                item_points(item, &mut got_pts);
            }
            let mut want_pts = Vec::new();
            for item in &want_layer.items {
                golden_item_points(item, &mut want_pts);
            }

            if got_pts.len() != want_pts.len() {
                failures.push(format!(
                    "  {} [{}]: {} shapes, want {}",
                    golden.name,
                    layer.name,
                    got_pts.len(),
                    want_pts.len()
                ));
                continue;
            }

            for (i, (got, want)) in got_pts.iter().zip(want_pts.iter()).enumerate() {
                if got.len() != want.len() {
                    failures.push(format!(
                        "  {} [{}] shape {i}: {} points, want {}",
                        golden.name,
                        layer.name,
                        got.len(),
                        want.len()
                    ));
                    break;
                }
                compared += got.len();
                for (j, (g, w)) in got.iter().zip(want.iter()).enumerate() {
                    if !close(g.x, w.x, tol) || !close(g.y, w.y, tol) {
                        failures.push(format!(
                            "  {} [{}] shape {i} point {j}: got ({}, {}), want ({}, {})",
                            golden.name, layer.name, g.x, g.y, w.x, w.y
                        ));
                        break;
                    }
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} layout divergences across {} cases:\n{}",
        failures.len(),
        goldens.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Guard against the comparison silently degenerating to nothing — an empty
    // point list on both sides would otherwise read as agreement.
    assert!(
        compared > 10_000,
        "only {compared} points compared; the flattening in this test is broken"
    );

    eprintln!(
        "{} layout cases matched ({compared} points compared)",
        goldens.len()
    );
}

#[test]
fn min_gap_removes_outline_points() {
    // The gap filter is what merges adjacent panels into a single cut. A large
    // threshold must drop points relative to a small one.
    let goldens = goldens_with_draw();
    let base = goldens
        .iter()
        .find(|g| g.name == "theta_cylindrical_m35_90")
        .expect("default cylindrical fixture");

    let mut tight = base.settings.to_input();
    tight.min_gap = 0.001;
    let mut loose = tight.clone();
    loose.min_gap = 0.05;

    let count = |input: &EllipsoidInput| {
        let g = compute_geometry(input);
        let f = compute_flat_geometry(&g, input);
        let scene = draw_edges(input, &f);
        match &scene.layer(LAYER_PATTERN).unwrap().items[0] {
            Item::Path { points, .. } => points.len(),
            other => panic!("expected the outline path, got {other:?}"),
        }
    };

    assert!(
        count(&loose) < count(&tight),
        "a larger min_gap should drop outline points: {} vs {}",
        count(&loose),
        count(&tight)
    );
}

#[test]
fn every_expected_layer_is_present() {
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
