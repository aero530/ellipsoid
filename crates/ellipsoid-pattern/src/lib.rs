//! 2D scene model and SVG emitter for ellipsoid flat patterns.
//!
//! This crate replaces paper.js. The original used only 13 of its symbols
//! (`Point`, `Color`, `Layer`, `Path`, `PointText`, `Shape.Rectangle`, `Group`,
//! and `exportSVG`) and none of its curve math, boolean ops, or hit-testing —
//! so a plain layered scene graph plus a serializer covers it.
//!
//! ```no_run
//! use ellipsoid_core::{EllipsoidInput, compute_flat_geometry, compute_geometry};
//! use ellipsoid_pattern::{build_scene, to_svg, SvgOptions};
//!
//! let input = EllipsoidInput::default();
//! let geometry = compute_geometry(&input);
//! let flat = compute_flat_geometry(&geometry, &input);
//! let scene = build_scene(&input, &geometry, &flat);
//! let svg = to_svg(&scene, &SvgOptions { inkscape_layers: input.inkscape_layers });
//! ```

pub mod cutouts;
pub mod domain;
pub mod layout;
pub mod scene;
pub mod svg;

pub use domain::{DomainTriangle, surface_domain};

pub use layout::{
    LAYER_BOUNDING_BOX, LAYER_CUTOUTS, LAYER_GUIDE_LINES, LAYER_NOTES, LAYER_PATTERN,
    LAYER_QUADS_DEST, LAYER_QUADS_SRC, PREVIEW_ONLY_LAYERS, PatternTransform, build_scene,
    draw_cutouts, draw_edges, draw_notes,
};
pub use scene::{Bounds, Color, Item, Layer, Scene, Stroke, Text, TextAnchor};
pub use svg::{SvgOptions, to_svg};
