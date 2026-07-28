//! A minimal layered 2D scene — the paper.js replacement.
//!
//! The original used exactly 13 paper.js symbols and none of its curve math,
//! boolean operations, or hit testing. What it actually needed was a list of
//! named layers holding styled polylines, rectangles, and text, plus bounds.
//! That is this module.
//!
//! All coordinates are SVG user units (pixels), already scaled by the drawing's
//! pixels-per-unit. Y points **down**, as in SVG.

use glam::DVec2;

/// An RGBA colour with components in `0.0..=1.0`, matching paper's `Color`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);

    /// `rgb(r,g,b)` with components rounded to bytes, as paper.js emits.
    pub fn to_svg(self) -> String {
        let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("rgb({},{},{})", byte(self.r), byte(self.g), byte(self.b))
    }
}

/// Stroke styling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
}

/// An axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: DVec2,
    pub max: DVec2,
}

impl Bounds {
    pub const EMPTY: Bounds = Bounds {
        min: DVec2::new(f64::INFINITY, f64::INFINITY),
        max: DVec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y
    }

    pub fn width(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.x - self.min.x
        }
    }

    pub fn height(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.y - self.min.y
        }
    }

    pub fn center(&self) -> DVec2 {
        (self.min + self.max) * 0.5
    }

    pub fn include(&mut self, p: DVec2) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn union(mut self, other: Bounds) -> Bounds {
        if other.is_empty() {
            return self;
        }
        self.include(other.min);
        self.include(other.max);
        self
    }
}

/// Where a [`Text`]'s `position` sits relative to its glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    /// `position` is the baseline origin, as paper's `PointText` point.
    BaselineStart,
    /// `position` is the centre of the text's bounding box.
    Center,
}

/// A run of text.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub position: DVec2,
    pub content: String,
    pub font_family: String,
    pub font_size: f64,
    pub fill: Color,
    pub anchor: TextAnchor,
    /// Rotation in degrees about `position`, clockwise (SVG convention).
    pub rotation_deg: f64,
}

/// Approximate advance width of a string.
///
/// A real shaper (`ab_glyph` with an embedded font) would be more faithful; see
/// `RUST_CONVERSION_PLAN.md` §7.2. This estimate is deterministic, which is what
/// makes exports byte-stable, and it only affects placement of the notes-layer
/// labels — never the cut geometry.
pub fn approx_text_width(content: &str, font_size: f64) -> f64 {
    // 0.6 em per character is a reasonable mean for both Roboto and the
    // Courier New used for the settings dump.
    content.chars().count() as f64 * font_size * 0.6
}

/// Approximate full line height, ascender to descender.
pub fn approx_text_height(font_size: f64) -> f64 {
    font_size * 1.2
}

impl Text {
    /// Unrotated bounds. Rotation is applied by [`Item::bounds`].
    fn upright_bounds(&self) -> Bounds {
        let w = approx_text_width(&self.content, self.font_size);
        let h = approx_text_height(self.font_size);
        match self.anchor {
            TextAnchor::BaselineStart => {
                // Baseline sits ~80% down the line box.
                let ascent = h * 0.8;
                Bounds {
                    min: DVec2::new(self.position.x, self.position.y - ascent),
                    max: DVec2::new(self.position.x + w, self.position.y + (h - ascent)),
                }
            }
            TextAnchor::Center => Bounds {
                min: self.position - DVec2::new(w, h) * 0.5,
                max: self.position + DVec2::new(w, h) * 0.5,
            },
        }
    }
}

/// A drawable.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A polyline, optionally closed.
    Path {
        points: Vec<DVec2>,
        closed: bool,
        stroke: Option<Stroke>,
        fill: Option<Color>,
    },
    /// An axis-aligned rectangle.
    Rect {
        bounds: Bounds,
        stroke: Option<Stroke>,
        fill: Option<Color>,
    },
    Text(Text),
    /// An unnamed grouping, emitted as a bare `<g>`.
    Group(Vec<Item>),
}

impl Item {
    pub fn bounds(&self) -> Bounds {
        match self {
            Item::Path { points, .. } => {
                let mut b = Bounds::EMPTY;
                for p in points {
                    b.include(*p);
                }
                b
            }
            Item::Rect { bounds, .. } => *bounds,
            Item::Text(text) => {
                let b = text.upright_bounds();
                if text.rotation_deg == 0.0 {
                    return b;
                }
                // Rotate the four corners about `position` and re-fit.
                let (sin, cos) = text.rotation_deg.to_radians().sin_cos();
                let mut out = Bounds::EMPTY;
                for corner in [
                    b.min,
                    DVec2::new(b.max.x, b.min.y),
                    b.max,
                    DVec2::new(b.min.x, b.max.y),
                ] {
                    let d = corner - text.position;
                    out.include(
                        text.position + DVec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos),
                    );
                }
                out
            }
            Item::Group(children) => children
                .iter()
                .fold(Bounds::EMPTY, |acc, c| acc.union(c.bounds())),
        }
    }
}

/// A named layer. Becomes a `<g id="...">`, which is what the Inkscape export
/// mode keys off.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub items: Vec<Item>,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            items: Vec::new(),
        }
    }

    pub fn bounds(&self) -> Bounds {
        self.items
            .iter()
            .fold(Bounds::EMPTY, |acc, i| acc.union(i.bounds()))
    }
}

/// A whole drawing: ordered layers, plus the canvas size.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub layers: Vec<Layer>,
    /// Canvas size in user units, from the `Bounding Box` layer.
    pub size: DVec2,
}

impl Scene {
    pub fn layer(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.name == name)
    }

    pub fn layer_mut(&mut self, name: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.name == name)
    }

    /// Drop a layer by name, returning whether it was present.
    ///
    /// The app removes both quadrilateral layers before exporting; see
    /// [`crate::layout::PREVIEW_ONLY_LAYERS`].
    pub fn remove_layer(&mut self, name: &str) -> bool {
        let before = self.layers.len();
        self.layers.retain(|l| l.name != name);
        self.layers.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bounds_have_zero_extent() {
        let b = Bounds::EMPTY;
        assert!(b.is_empty());
        assert_eq!(b.width(), 0.0);
        assert_eq!(b.height(), 0.0);
    }

    #[test]
    fn path_bounds_cover_every_point() {
        let item = Item::Path {
            points: vec![
                DVec2::new(1.0, 2.0),
                DVec2::new(-3.0, 5.0),
                DVec2::new(0.0, -1.0),
            ],
            closed: false,
            stroke: None,
            fill: None,
        };
        let b = item.bounds();
        assert_eq!(b.min, DVec2::new(-3.0, -1.0));
        assert_eq!(b.max, DVec2::new(1.0, 5.0));
    }

    #[test]
    fn rotating_text_by_90_swaps_its_extent() {
        let upright = Text {
            position: DVec2::ZERO,
            content: "hello world".into(),
            font_family: "Roboto".into(),
            font_size: 10.0,
            fill: Color::BLACK,
            anchor: TextAnchor::Center,
            rotation_deg: 0.0,
        };
        let rotated = Text {
            rotation_deg: -90.0,
            ..upright.clone()
        };

        let a = Item::Text(upright).bounds();
        let b = Item::Text(rotated).bounds();
        assert!((a.width() - b.height()).abs() < 1e-9);
        assert!((a.height() - b.width()).abs() < 1e-9);
    }

    #[test]
    fn group_bounds_are_the_union_of_children() {
        let group = Item::Group(vec![
            Item::Path {
                points: vec![DVec2::ZERO, DVec2::new(1.0, 1.0)],
                closed: false,
                stroke: None,
                fill: None,
            },
            Item::Path {
                points: vec![DVec2::new(5.0, -2.0)],
                closed: false,
                stroke: None,
                fill: None,
            },
        ]);
        let b = group.bounds();
        assert_eq!(b.min, DVec2::new(0.0, -2.0));
        assert_eq!(b.max, DVec2::new(5.0, 1.0));
    }

    #[test]
    fn colors_serialize_as_byte_rgb() {
        assert_eq!(Color::rgb(1.0, 0.0, 0.5).to_svg(), "rgb(255,0,128)");
        assert_eq!(Color::BLACK.to_svg(), "rgb(0,0,0)");
    }

    #[test]
    fn removing_a_layer_reports_whether_it_existed() {
        let mut scene = Scene {
            layers: vec![Layer::new("a"), Layer::new("b")],
            size: DVec2::ZERO,
        };
        assert!(scene.remove_layer("a"));
        assert!(!scene.remove_layer("a"));
        assert_eq!(scene.layers.len(), 1);
    }
}
