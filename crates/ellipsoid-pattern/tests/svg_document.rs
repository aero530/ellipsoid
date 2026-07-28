//! End-to-end checks on the exported SVG document.
//!
//! Parity of the underlying geometry is covered by `layout_parity.rs`; this is
//! about the document being valid, self-consistent, and structured the way
//! Inkscape expects.

use ellipsoid_core::{EllipsoidInput, Projection, compute_flat_geometry, compute_geometry};
use ellipsoid_pattern::{
    LAYER_BOUNDING_BOX, LAYER_GUIDE_LINES, LAYER_NOTES, LAYER_PATTERN, PREVIEW_ONLY_LAYERS,
    SvgOptions, build_scene, to_svg,
};

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

/// Minimal well-formedness check: every `<tag` opens and closes, quotes balance.
///
/// Not a full XML parser — enough to catch unescaped content and unclosed
/// elements without taking a dependency for a test.
fn assert_well_formed(svg: &str) {
    let mut depth = 0i32;
    let mut rest = svg;
    while let Some(open) = rest.find('<') {
        rest = &rest[open..];
        let close = rest
            .find('>')
            .unwrap_or_else(|| panic!("unclosed tag near {:?}", &rest[..40.min(rest.len())]));
        let tag = &rest[1..close];

        if tag.starts_with('?') || tag.starts_with('!') {
            // declaration or comment
        } else if let Some(name) = tag.strip_prefix('/') {
            depth -= 1;
            assert!(depth >= 0, "closing </{name}> with nothing open");
        } else if !tag.ends_with('/') {
            depth += 1;
        }

        assert_eq!(
            tag.matches('"').count() % 2,
            0,
            "unbalanced quotes in <{tag}>"
        );
        rest = &rest[close + 1..];
    }
    assert_eq!(depth, 0, "unbalanced elements: depth {depth} at end");
}

#[test]
fn document_is_well_formed_for_every_projection() {
    for projection in Projection::ALL {
        let input = EllipsoidInput {
            projection,
            ..Default::default()
        };
        let svg = render(&input);
        assert_well_formed(&svg);
        assert!(svg.starts_with("<?xml"), "missing declaration");
        assert!(svg.trim_end().ends_with("</svg>"), "missing root close");
    }
}

#[test]
fn export_drops_the_preview_only_layers() {
    // The app removed both quadrilateral layers before exporting; they exist
    // only to carry panel-mapping data for later features.
    let svg = render(&EllipsoidInput::default());
    for name in PREVIEW_ONLY_LAYERS {
        assert!(
            !svg.contains(&format!(r#"id="{name}""#)),
            "{name} leaked into the export"
        );
    }
    for name in [LAYER_PATTERN, LAYER_BOUNDING_BOX, LAYER_GUIDE_LINES] {
        assert!(svg.contains(&format!(r#"id="{name}""#)), "missing {name}");
    }
}

#[test]
fn notes_appear_only_when_there_is_margin_for_them() {
    let with_margin = EllipsoidInput {
        image_offset: 0.5,
        ..Default::default()
    };
    let without = EllipsoidInput {
        image_offset: 0.25,
        ..Default::default()
    };
    assert!(render(&with_margin).contains(&format!(r#"id="{LAYER_NOTES}""#)));
    assert!(!render(&without).contains(&format!(r#"id="{LAYER_NOTES}""#)));
}

#[test]
fn notes_carry_the_filename_and_a_ruler() {
    let input = EllipsoidInput::default();
    let svg = render(&input);
    assert!(
        svg.contains(&format!("{}.svg", input.filename_stem())),
        "filename label missing"
    );
    // Ruler labels are `0in`, `1in`, ...
    assert!(svg.contains(">0in<"), "ruler origin label missing");
    assert!(svg.contains(">1in<"), "ruler is not graduated");
}

#[test]
fn inkscape_mode_is_opt_in() {
    let plain = render(&EllipsoidInput {
        inkscape_layers: false,
        ..Default::default()
    });
    let tagged = render(&EllipsoidInput {
        inkscape_layers: true,
        ..Default::default()
    });

    // Check for markup, not the bare word: the notes layer stamps the settings
    // as JSON, which legitimately contains the key `inkscape_layers`.
    assert!(
        !plain.contains("xmlns:inkscape"),
        "plain SVG declared the inkscape namespace"
    );
    assert!(
        !plain.contains("inkscape:groupmode"),
        "plain SVG tagged a layer"
    );
    assert!(tagged.contains(r#"inkscape:groupmode="layer""#));
    assert!(
        tagged.contains("xmlns:inkscape"),
        "namespace must be declared or the file is invalid XML"
    );
}

#[test]
fn canvas_scales_with_the_unit() {
    // The pattern is the same physical size in every unit, so the pixel canvas
    // must scale with pixels-per-unit.
    let inches = EllipsoidInput::default();
    let mm = EllipsoidInput {
        unit: ellipsoid_core::Unit::Mm,
        ..Default::default()
    };

    let size = |input: &EllipsoidInput| {
        let g = compute_geometry(input);
        let f = compute_flat_geometry(&g, input);
        build_scene(input, &g, &f).size
    };

    let ratio = size(&inches).x / size(&mm).x;
    let expected = inches.px_per_unit() / mm.px_per_unit();
    assert!(
        (ratio - expected).abs() < 1e-9,
        "canvas ratio {ratio} != ppu ratio {expected}"
    );
}

/// The document must be the size of the pattern, not the size of its labels.
///
/// The canvas is the content bounds (matching paper.js's `bounds: 'content'`),
/// so an over-long notes stamp would silently pad the page — which it did, at
/// 1.6x, until the stamp was made to shrink to fit. The vertical overhang of a
/// few px is expected: the settings text sits partly above y=0, exactly as in
/// the original.
#[test]
fn canvas_tracks_the_pattern_not_the_notes() {
    for projection in Projection::ALL {
        let input = EllipsoidInput {
            projection,
            ..Default::default()
        };
        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);
        let scene = build_scene(&input, &geometry, &flat);

        let box_bounds = scene.layer(LAYER_BOUNDING_BOX).unwrap().bounds();
        let content = scene
            .layers
            .iter()
            .fold(ellipsoid_pattern::Bounds::EMPTY, |acc, l| {
                acc.union(l.bounds())
            });

        assert!(
            content.width() <= box_bounds.width() + 1.0,
            "{projection}: canvas {} wider than the pattern {}",
            content.width(),
            box_bounds.width()
        );
        assert!(
            content.height() <= box_bounds.height() + 0.3 * input.px_per_unit(),
            "{projection}: canvas {} much taller than the pattern {}",
            content.height(),
            box_bounds.height()
        );
    }
}

#[test]
fn export_is_byte_stable() {
    let input = EllipsoidInput::default();
    assert_eq!(render(&input), render(&input));
}
