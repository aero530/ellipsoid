//! SVG serialisation — replaces paper.js `exportSVG`.
//!
//! Output is deterministic: fixed precision, fixed attribute order, no
//! timestamps or generated ids. Two exports of the same pattern diff cleanly.

use std::fmt::Write as _;

use glam::DVec2;

use crate::scene::{Bounds, Color, Item, Scene, Stroke, TextAnchor};

/// Decimal places for coordinates.
///
/// At the default 96 px/in this is ~1e-5 in — far below any cutter's
/// resolution. The original rounded the cut outline to 3 places by construction
/// and let paper.js use its own precision elsewhere; one setting for everything
/// is simpler and no less accurate.
const PRECISION: usize = 3;

const INKSCAPE_NS: &str = "http://www.inkscape.org/namespaces/inkscape";
const SODIPODI_NS: &str = "http://sodipodi.sourceforge.net/DTD/sodipodi-0.0.dtd";

/// Export options.
#[derive(Debug, Clone, Copy, Default)]
pub struct SvgOptions {
    /// Tag each named layer so Inkscape treats it as a layer rather than a
    /// plain group.
    pub inkscape_layers: bool,
}

/// Format a coordinate: fixed precision, trailing zeros trimmed, no `-0`.
fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let mut s = format!("{v:.PRECISION$}");
    if s.contains('.') {
        s.truncate(s.trim_end_matches('0').trim_end_matches('.').len());
    }
    if s == "-0" || s.is_empty() {
        s = "0".into();
    }
    s
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn paint_attrs(out: &mut String, stroke: Option<Stroke>, fill: Option<Color>) {
    match fill {
        Some(c) => {
            let _ = write!(out, r#" fill="{}""#, c.to_svg());
            if c.a < 1.0 {
                let _ = write!(out, r#" fill-opacity="{}""#, num(c.a));
            }
        }
        None => out.push_str(r#" fill="none""#),
    }
    if let Some(s) = stroke {
        let _ = write!(out, r#" stroke="{}""#, s.color.to_svg());
        let _ = write!(out, r#" stroke-width="{}""#, num(s.width));
        if s.color.a < 1.0 {
            let _ = write!(out, r#" stroke-opacity="{}""#, num(s.color.a));
        }
    }
}

fn path_data(points: &[DVec2], closed: bool) -> String {
    let mut d = String::with_capacity(points.len() * 16);
    for (i, p) in points.iter().enumerate() {
        if i > 0 {
            d.push(' ');
        }
        let _ = write!(
            d,
            "{}{},{}",
            if i == 0 { 'M' } else { 'L' },
            num(p.x),
            num(p.y)
        );
    }
    if closed {
        d.push_str(" Z");
    }
    d
}

fn write_item(out: &mut String, item: &Item, indent: usize) {
    let pad = "  ".repeat(indent);
    match item {
        Item::Path {
            points,
            closed,
            stroke,
            fill,
        } => {
            if points.is_empty() {
                return;
            }
            let _ = write!(out, r#"{pad}<path d="{}""#, path_data(points, *closed));
            paint_attrs(out, *stroke, *fill);
            out.push_str("/>\n");
        }
        Item::Rect {
            bounds,
            stroke,
            fill,
        } => {
            let _ = write!(
                out,
                r#"{pad}<rect x="{}" y="{}" width="{}" height="{}""#,
                num(bounds.min.x),
                num(bounds.min.y),
                num(bounds.width()),
                num(bounds.height())
            );
            paint_attrs(out, *stroke, *fill);
            out.push_str("/>\n");
        }
        Item::Text(text) => {
            let _ = write!(
                out,
                r#"{pad}<text x="{}" y="{}" font-family="{}" font-size="{}" fill="{}""#,
                num(text.position.x),
                num(text.position.y),
                escape(&text.font_family),
                num(text.font_size),
                text.fill.to_svg()
            );
            if text.anchor == TextAnchor::Center {
                out.push_str(r#" text-anchor="middle" dominant-baseline="central""#);
            }
            if text.rotation_deg != 0.0 {
                let _ = write!(
                    out,
                    r#" transform="rotate({},{},{})""#,
                    num(text.rotation_deg),
                    num(text.position.x),
                    num(text.position.y)
                );
            }
            let _ = writeln!(out, ">{}</text>", escape(&text.content));
        }
        Item::Group(children) => {
            let _ = writeln!(out, "{pad}<g>");
            for child in children {
                write_item(out, child, indent + 1);
            }
            let _ = writeln!(out, "{pad}</g>");
        }
    }
}

/// Serialise `scene` to a standalone SVG document.
pub fn to_svg(scene: &Scene, options: &SvgOptions) -> String {
    // paper.js exported with `bounds: 'content'`, so the viewBox hugs the
    // drawing rather than an arbitrary canvas.
    let content = scene
        .layers
        .iter()
        .fold(Bounds::EMPTY, |acc, l| acc.union(l.bounds()));
    let content = if content.is_empty() {
        Bounds {
            min: DVec2::ZERO,
            max: scene.size,
        }
    } else {
        content
    };

    let mut out = String::with_capacity(64 * 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    out.push_str(" xmlns:xlink=\"http://www.w3.org/1999/xlink\"");
    if options.inkscape_layers {
        let _ = write!(out, " xmlns:inkscape=\"{INKSCAPE_NS}\"");
        let _ = write!(out, " xmlns:sodipodi=\"{SODIPODI_NS}\"");
    }
    let _ = writeln!(
        out,
        " width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">",
        num(content.width()),
        num(content.height()),
        num(content.min.x),
        num(content.min.y),
        num(content.width()),
        num(content.height())
    );

    for layer in &scene.layers {
        let _ = write!(out, "  <g id=\"{}\"", escape(&layer.name));
        if options.inkscape_layers {
            // Matches the rewrite the original applied to its exported string.
            let _ = write!(
                out,
                " inkscape:groupmode=\"layer\" inkscape:label=\"{}\"",
                escape(&layer.name)
            );
        }
        out.push_str(">\n");
        for item in &layer.items {
            write_item(&mut out, item, 2);
        }
        out.push_str("  </g>\n");
    }

    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Layer, Stroke};

    fn sample() -> Scene {
        let mut layer = Layer::new("Ellipsoid Pattern");
        layer.items.push(Item::Path {
            points: vec![DVec2::ZERO, DVec2::new(10.0, 0.0), DVec2::new(10.0, 5.0)],
            closed: true,
            stroke: Some(Stroke {
                color: Color::BLACK,
                width: 1.5,
            }),
            fill: Some(Color::rgba(1.0, 1.0, 1.0, 0.3)),
        });
        Scene {
            layers: vec![layer],
            size: DVec2::new(10.0, 5.0),
        }
    }

    #[test]
    fn numbers_are_trimmed_and_signed_zero_is_normalised() {
        assert_eq!(num(1.0), "1");
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(1.23456), "1.235");
        assert_eq!(num(-0.0001), "0");
        assert_eq!(num(0.0), "0");
    }

    #[test]
    fn output_is_byte_stable() {
        let scene = sample();
        let opts = SvgOptions::default();
        assert_eq!(to_svg(&scene, &opts), to_svg(&scene, &opts));
    }

    #[test]
    fn layers_become_identified_groups() {
        let svg = to_svg(&sample(), &SvgOptions::default());
        assert!(svg.contains(r#"<g id="Ellipsoid Pattern">"#), "{svg}");
        assert!(!svg.contains("inkscape"), "plain mode must stay clean");
    }

    #[test]
    fn inkscape_mode_tags_layers_and_declares_the_namespace() {
        let svg = to_svg(
            &sample(),
            &SvgOptions {
                inkscape_layers: true,
            },
        );
        assert!(svg.contains(&format!(r#"xmlns:inkscape="{INKSCAPE_NS}""#)));
        assert!(svg.contains(r#"inkscape:groupmode="layer""#));
    }

    #[test]
    fn closed_paths_end_with_z() {
        let svg = to_svg(&sample(), &SvgOptions::default());
        assert!(svg.contains(r#"d="M0,0 L10,0 L10,5 Z""#), "{svg}");
    }

    #[test]
    fn translucent_fill_emits_separate_opacity() {
        let svg = to_svg(&sample(), &SvgOptions::default());
        assert!(svg.contains(r#"fill="rgb(255,255,255)""#), "{svg}");
        assert!(svg.contains(r#"fill-opacity="0.3""#), "{svg}");
    }

    #[test]
    fn text_content_is_escaped() {
        let mut layer = Layer::new("Notes");
        layer.items.push(Item::Text(crate::scene::Text {
            position: DVec2::ZERO,
            content: r#"a<b>&"c""#.into(),
            font_family: "Roboto".into(),
            font_size: 10.0,
            fill: Color::BLACK,
            anchor: TextAnchor::BaselineStart,
            rotation_deg: 0.0,
        }));
        let scene = Scene {
            layers: vec![layer],
            size: DVec2::ONE,
        };
        let svg = to_svg(&scene, &SvgOptions::default());
        assert!(svg.contains("a&lt;b&gt;&amp;&quot;c&quot;"), "{svg}");
        assert!(!svg.contains("<b>"));
    }
}
