//! Flat-pattern layout — port of `drawEdges` and `drawNotes` in
//! `app/utils/ellipsoid.js`.
//!
//! Produces a [`Scene`] in SVG user units. Everything is scaled by the
//! drawing's pixels-per-unit here, so downstream code never rescales.

// As in `ellipsoid-core`: index loops are kept so this reads against the
// JavaScript line by line.
#![allow(clippy::needless_range_loop)]

use ellipsoid_core::flatten::FlatGeometry;
use ellipsoid_core::geometry::Geometry;
use ellipsoid_core::rotate::distance;
use ellipsoid_core::surface::SurfaceParam;
use ellipsoid_core::{DVec3, EllipsoidInput, Projection};
use glam::DVec2;

use crate::scene::{
    Bounds, Color, Item, Layer, Scene, Stroke, Text, TextAnchor, approx_text_width,
};

pub const LAYER_PATTERN: &str = "Ellipsoid Pattern";
pub const LAYER_BOUNDING_BOX: &str = "Bounding Box";
pub const LAYER_GUIDE_LINES: &str = "Guide Lines";
pub const LAYER_QUADS_DEST: &str = "Edges Destination Quadrilaterals";
pub const LAYER_QUADS_SRC: &str = "Edges Source Quadrilaterals";
pub const LAYER_CUTOUTS: &str = "Cutouts";
pub const LAYER_NOTES: &str = "Notes";

/// Layers the app drops before exporting ([scene.js:128-129]).
///
/// They exist only so the panel-to-panel mapping data is available — the
/// deferred texture feature needs it, and so will cutouts (plan §7.3/§7.4).
pub const PREVIEW_ONLY_LAYERS: [&str; 2] = [LAYER_QUADS_DEST, LAYER_QUADS_SRC];

/// Placement of flat-pattern coordinates into the output image.
///
/// Shared by [`draw_edges`] and [`draw_cutouts`] so holes land on the cut
/// outline rather than a differently-transformed copy of it.
pub struct PatternTransform {
    pub ppu: f64,
    pub shift: DVec2,
    /// Canvas size in SVG user units.
    pub image: DVec2,
}

impl PatternTransform {
    pub fn new(input: &EllipsoidInput, flat: &FlatGeometry) -> Self {
        let ppu = input.px_per_unit();
        let (min, max) = ellipsoid_core::flatten::edge_extents(&flat.edges_flat);

        let shift = match input.projection {
            Projection::Spherical => DVec2::new(
                (max.x - min.x) / 2.0 + input.image_offset,
                (max.y - min.y) / 2.0 + input.image_offset,
            ),
            Projection::Cylindrical => {
                DVec2::new(min.x.abs() + input.image_offset, input.image_offset + max.y)
            }
        };

        Self {
            ppu,
            shift,
            image: DVec2::new(
                (max.x - min.x + 2.0 * input.image_offset) * ppu,
                (max.y - min.y + 2.0 * input.image_offset) * ppu,
            ),
        }
    }

    /// Line weight, scaled with the drawing so it stays unit-relative.
    pub fn stroke_width(&self) -> f64 {
        3.0 * self.ppu / 90.0
    }

    /// Guide lines and both quad layers place points with y *negated*.
    pub fn place(&self, p: DVec3) -> DVec2 {
        DVec2::new(
            (self.shift.x + p.x) * self.ppu,
            (self.shift.y - p.y) * self.ppu,
        )
    }

    /// Where the cut outline and the cutouts go.
    ///
    /// Now identical to [`Self::place`], and kept as a separate name only
    /// because the distinction is what plan §8.7 was about: the original added
    /// y in the spherical branch and subtracted it in the cylindrical one,
    /// while the guide lines, both quadrilateral layers and the notes ruler
    /// always subtracted. That mirrored the spherical outline against every
    /// other layer in its own drawing.
    ///
    /// The mirror is invisible in the outline itself — a star of petals with one
    /// on the axis is symmetric about that axis, so reflecting it maps it onto
    /// itself — which is why it survived the port unnoticed. It was *not*
    /// invisible for cutouts, which are placed through here: a hole reflected
    /// about the centre lands on the mirror-image petal, at the mirror-image
    /// height within it. Congruent petals made that look plausible and cut
    /// wrong.
    pub fn place_outline(&self, p: DVec3) -> DVec2 {
        self.place(p)
    }

    /// The inverse of [`Self::place_outline`]: a point on the page back to flat
    /// pattern coordinates.
    ///
    /// Only `x` and `y` can be recovered — `z` is not drawn, so the page does
    /// not carry it. That is enough for [`ellipsoid_core::surface::flat_to_surface`],
    /// which searches in the page plane.
    pub fn unplace_outline(&self, page: DVec2) -> DVec2 {
        DVec2::new(
            page.x / self.ppu - self.shift.x,
            self.shift.y - page.y / self.ppu,
        )
    }
}

/// Build the scene exactly as the app does: lay out the pattern, drop the
/// preview-only layers, add cutouts, then notes if there is margin for them.
pub fn build_scene(input: &EllipsoidInput, geometry: &Geometry, flat: &FlatGeometry) -> Scene {
    let mut scene = draw_edges(input, flat);
    for name in PREVIEW_ONLY_LAYERS {
        scene.remove_layer(name);
    }
    draw_cutouts(&mut scene, input, geometry, flat);
    // The original gated on the same threshold, via a stringified value that
    // JavaScript coerced back to a number.
    if input.image_offset >= 0.5 {
        draw_notes(&mut scene, input);
    }
    scene
}

/// Lay out the cut outline, bounding box, guide lines, and both quad layers.
pub fn draw_edges(input: &EllipsoidInput, flat: &FlatGeometry) -> Scene {
    let transform = PatternTransform::new(input, flat);
    let ppu = transform.ppu;
    let shift = transform.shift;
    let image = transform.image;
    let stroke_width = transform.stroke_width();

    let projection = input.projection;
    let h_top = input.h_top;
    let min_gap = input.min_gap;

    let e = &flat.edges_flat;
    let widest_row = flat.widest_row;
    let phi_divisions = e.len();
    let theta_divisions = e[0].len() - 1;

    let place = |p: DVec3| transform.place(p);

    // -------------------------------------------------------------------
    // Cut outline
    //
    // Walk the strips emitting the previous strip's trailing edge then this
    // strip's leading edge, so the result traces the cut outline rather than
    // the panel edges themselves. Points closer together than `min_gap` are
    // dropped, which merges adjacent panels into one cut.
    // -------------------------------------------------------------------
    let mut outline: Vec<DVec2> = Vec::new();

    match projection {
        Projection::Spherical => {
            for ip in 0..phi_divisions {
                let prev = if ip == 0 { phi_divisions - 1 } else { ip - 1 };

                for it in 0..=theta_divisions {
                    // The gap test degenerates when height is added on top, so
                    // that rung is always kept.
                    let keep = (h_top > 0.0 && it == theta_divisions)
                        || distance(e[prev][it][1], e[ip][it][0]) > min_gap;
                    if keep {
                        let p = e[prev][it][1];
                        outline.push(transform.place_outline(p));
                    }
                }
                for it in (0..=theta_divisions).rev() {
                    let keep = (h_top > 0.0 && it == theta_divisions)
                        || distance(e[prev][it][1], e[ip][it][0]) > min_gap;
                    if keep {
                        let p = e[ip][it][0];
                        outline.push(transform.place_outline(p));
                    }
                }
            }
        }
        Projection::Cylindrical => {
            // Down one side of every strip, then back up the other.
            for ip in 0..phi_divisions {
                let prev = if ip == 0 { phi_divisions - 1 } else { ip - 1 };
                let next = if ip + 1 == phi_divisions { 0 } else { ip + 1 };

                for it in (0..=widest_row).rev() {
                    if distance(e[prev][it][1], e[ip][it][0]) > min_gap {
                        outline.push(place(e[ip][it][0]));
                    }
                }
                for it in 0..=widest_row {
                    if distance(e[next][it][0], e[ip][it][1]) > min_gap {
                        outline.push(place(e[ip][it][1]));
                    }
                }
            }
            for ip in (0..phi_divisions).rev() {
                let prev = if ip + 1 == phi_divisions { 0 } else { ip + 1 };
                let next = if ip == 0 { phi_divisions - 1 } else { ip - 1 };

                for it in widest_row..=theta_divisions {
                    if distance(e[prev][it][0], e[ip][it][1]) > min_gap {
                        outline.push(place(e[ip][it][1]));
                    }
                }
                for it in ((widest_row + 1)..=theta_divisions).rev() {
                    if distance(e[next][it][1], e[ip][it][0]) > min_gap {
                        outline.push(place(e[ip][it][0]));
                    }
                }
            }
        }
    }

    let mut pattern_layer = Layer::new(LAYER_PATTERN);
    pattern_layer.items.push(Item::Path {
        points: outline,
        closed: true,
        stroke: Some(Stroke {
            color: Color::BLACK,
            width: stroke_width * 0.5,
        }),
        fill: Some(Color::rgba(1.0, 1.0, 1.0, 0.3)),
    });

    // -------------------------------------------------------------------
    // Bounding box
    // -------------------------------------------------------------------
    let mut bounding_layer = Layer::new(LAYER_BOUNDING_BOX);
    bounding_layer.items.push(Item::Rect {
        bounds: Bounds {
            min: DVec2::ZERO,
            max: image,
        },
        stroke: Some(Stroke {
            color: Color::rgb(0.2, 0.2, 0.2), // '#333333'
            width: 0.01 * ppu,
        }),
        // Fully transparent, but paper still emitted the attribute.
        fill: Some(Color::rgba(1.0, 0.0, 0.5, 0.0)),
    });

    // -------------------------------------------------------------------
    // Guide lines — fold/glue alignment across each rung
    // -------------------------------------------------------------------
    let guide_stroke = Some(Stroke {
        color: Color::rgb(0.0, 1.0, 0.0),
        width: stroke_width * 0.25,
    });
    let mut guides = Vec::with_capacity(phi_divisions * theta_divisions);
    for ip in 0..phi_divisions {
        for it in 1..=theta_divisions {
            guides.push(Item::Path {
                points: vec![place(e[ip][it][0]), place(e[ip][it][1])],
                closed: false,
                stroke: guide_stroke,
                fill: None,
            });
        }
    }
    let mut guide_layer = Layer::new(LAYER_GUIDE_LINES);
    // The original wrapped these in a Group inside the layer.
    guide_layer.items.push(Item::Group(guides));

    // -------------------------------------------------------------------
    // Destination quadrilaterals — one per panel face, in pattern space
    // -------------------------------------------------------------------
    let quad_stroke = |color: Color| {
        Some(Stroke {
            color,
            width: stroke_width * 0.25,
        })
    };

    let mut dest_layer = Layer::new(LAYER_QUADS_DEST);
    for ip in 0..phi_divisions {
        for it in 0..theta_divisions {
            dest_layer.items.push(Item::Path {
                points: vec![
                    place(e[ip][it][0]),
                    place(e[ip][it + 1][0]),
                    place(e[ip][it + 1][1]),
                    place(e[ip][it][1]),
                ],
                closed: true,
                stroke: quad_stroke(Color::rgb(1.0, 0.0, 0.0)),
                fill: None,
            });
        }
    }

    // -------------------------------------------------------------------
    // Source quadrilaterals — the same faces averaged with their neighbours,
    // giving a continuous mapping across seams. The first and last strips join
    // each other, so their shared edge is forced straight.
    // -------------------------------------------------------------------
    let left_x = e[0][widest_row][0].x;
    let right_x = e[phi_divisions - 1][widest_row][1].x;
    let mid = |a: DVec2, b: DVec2| (a + b) * 0.5;

    let mut src_layer = Layer::new(LAYER_QUADS_SRC);
    for ip in 0..phi_divisions {
        for it in 0..theta_divisions {
            let (p1a, p1b, p2a, p2b, p3a, p3b, p4a, p4b);

            if ip == 0 {
                match projection {
                    Projection::Spherical => {
                        p1a = place(e[ip][it][0]);
                        p1b = place(e[phi_divisions - 1][it][1]);
                        p2a = place(e[ip][it + 1][0]);
                        p2b = place(e[phi_divisions - 1][it + 1][1]);
                    }
                    Projection::Cylindrical => {
                        // Straight seam at the left edge.
                        let a =
                            DVec2::new((shift.x + left_x) * ppu, (shift.y - e[ip][it][0].y) * ppu);
                        let b = DVec2::new(
                            (shift.x + left_x) * ppu,
                            (shift.y - e[ip][it + 1][0].y) * ppu,
                        );
                        p1a = a;
                        p1b = a;
                        p2a = b;
                        p2b = b;
                    }
                }
                p3a = place(e[ip][it + 1][1]);
                p3b = place(e[ip + 1][it + 1][0]);
                p4a = place(e[ip][it][1]);
                p4b = place(e[ip + 1][it][0]);
            } else if ip == phi_divisions - 1 {
                p1a = place(e[ip][it][0]);
                p1b = place(e[ip - 1][it][1]);
                p2a = place(e[ip][it + 1][0]);
                p2b = place(e[ip - 1][it + 1][1]);
                match projection {
                    Projection::Spherical => {
                        p3a = place(e[ip][it + 1][1]);
                        p3b = place(e[0][it + 1][0]);
                        p4a = place(e[ip][it][1]);
                        p4b = place(e[0][it][0]);
                    }
                    Projection::Cylindrical => {
                        // Straight seam at the right edge.
                        let a = DVec2::new(
                            (shift.x + right_x) * ppu,
                            (shift.y - e[ip][it + 1][1].y) * ppu,
                        );
                        let b =
                            DVec2::new((shift.x + right_x) * ppu, (shift.y - e[ip][it][1].y) * ppu);
                        p3a = a;
                        p3b = a;
                        p4a = b;
                        p4b = b;
                    }
                }
            } else {
                p1a = place(e[ip][it][0]);
                p1b = place(e[ip - 1][it][1]);
                p2a = place(e[ip][it + 1][0]);
                p2b = place(e[ip - 1][it + 1][1]);
                p3a = place(e[ip][it + 1][1]);
                p3b = place(e[ip + 1][it + 1][0]);
                p4a = place(e[ip][it][1]);
                p4b = place(e[ip + 1][it][0]);
            }

            src_layer.items.push(Item::Path {
                points: vec![mid(p1a, p1b), mid(p2a, p2b), mid(p3a, p3b), mid(p4a, p4b)],
                closed: true,
                stroke: quad_stroke(Color::rgb(1.0, 0.0, 1.0)),
                fill: None,
            });
        }
    }

    Scene {
        layers: vec![
            pattern_layer,
            bounding_layer,
            guide_layer,
            dest_layer,
            src_layer,
        ],
        size: image,
    }
}

/// Remove the cutouts from the pattern.
///
/// A cutout inside a panel becomes a closed ring in the `Cutouts` layer. One
/// that reaches a panel edge instead **opens that edge up**: the cut outline
/// detours around it and no chord is cut across the seam, so the halves on
/// either side form the shape once the panels are joined.
///
/// Both fall out of one boolean difference — which of the two a cutout gets is
/// decided by whether it touches the boundary, not by a special case here.
pub fn draw_cutouts(
    scene: &mut Scene,
    input: &EllipsoidInput,
    geometry: &Geometry,
    flat: &FlatGeometry,
) {
    if input.cutouts.is_empty() {
        return;
    }

    let transform = PatternTransform::new(input, flat);
    let param = SurfaceParam::new(geometry);

    let pieces: Vec<Vec<DVec2>> = input
        .cutouts
        .iter()
        .flat_map(|cutout| crate::cutouts::pieces(&param, geometry, flat, cutout))
        .map(|piece| {
            piece
                .into_iter()
                .map(|p| transform.place_outline(p))
                .collect()
        })
        .collect();
    if pieces.is_empty() {
        return;
    }

    let stroke_width = transform.stroke_width();

    // Guide lines are fold and glue marks, so they have no business crossing a
    // hole: there is no material there to fold. Trim before touching the
    // outline, while `pieces` is still the only borrow in play.
    if let Some(guides) = scene.layer_mut(LAYER_GUIDE_LINES) {
        for item in &mut guides.items {
            trim_guides(item, &pieces);
        }
    }

    // Take the cut outline out of the pattern layer, keeping its styling.
    let Some(pattern) = scene.layer_mut(LAYER_PATTERN) else {
        return;
    };
    let Some(Item::Path {
        points: outline,
        stroke,
        fill,
        ..
    }) = pattern.items.first().cloned()
    else {
        return;
    };

    let (mut outer, mut holes) = crate::cutouts::subtract_from_outline(&outline, &pieces);

    // Weld the seams the boolean left pinched — see `cutouts::despike`. A
    // feature thinner than the line drawing it cannot be cut, which is the same
    // threshold the ring filter below uses.
    for ring in outer.iter_mut().chain(holes.iter_mut()) {
        crate::cutouts::despike(ring, stroke_width);
    }

    // Drop anything smaller than the line that would draw it.
    //
    // Where two panels part company along a seam, subtracting a shape that
    // crosses it leaves a triangle a few thousandths of an inch across. It is
    // real geometry, not a rounding error, but it is well under any cutter's
    // kerf and shows up only as a stray tick on the drawing.
    let worth_cutting =
        |ring: &Vec<DVec2>| crate::cutouts::area(ring) > stroke_width * stroke_width;
    let kept: Vec<Vec<DVec2>> = outer.iter().filter(|r| worth_cutting(r)).cloned().collect();
    // ...unless that would throw the pattern away.
    let outer = if kept.is_empty() { outer } else { kept };
    let holes: Vec<Vec<DVec2>> = holes.into_iter().filter(worth_cutting).collect();

    pattern.items.clear();
    for ring in outer {
        pattern.items.push(Item::Path {
            points: ring,
            closed: true,
            stroke,
            fill,
        });
    }

    if holes.is_empty() {
        return;
    }
    let mut layer = Layer::new(LAYER_CUTOUTS);
    for ring in holes {
        layer.items.push(Item::Path {
            points: ring,
            closed: true,
            stroke: Some(Stroke {
                color: Color::BLACK,
                width: stroke_width * 0.5,
            }),
            fill: None,
        });
    }
    scene.layers.push(layer);
}

/// Replace every guide segment with the parts of it that miss the cutouts.
///
/// Guides arrive wrapped in a `Group`, as the original had them, so this
/// recurses. A segment swallowed whole simply disappears.
fn trim_guides(item: &mut Item, pieces: &[Vec<DVec2>]) {
    match item {
        Item::Group(children) => {
            let trimmed: Vec<Item> = children
                .drain(..)
                .flat_map(|mut child| {
                    trim_guides(&mut child, pieces);
                    match child {
                        // A segment that split into several is spliced back in.
                        Item::Group(parts) => parts,
                        other => vec![other],
                    }
                })
                .collect();
            *children = trimmed;
        }
        Item::Path { points, stroke, .. } if points.len() == 2 => {
            let parts = crate::cutouts::outside_pieces(points[0], points[1], pieces);
            let stroke = *stroke;
            // One surviving part in place; none or several via a group, which
            // the arm above flattens on the way out.
            match parts.len() {
                1 => *points = parts[0].to_vec(),
                _ => {
                    *item = Item::Group(
                        parts
                            .into_iter()
                            .map(|p| Item::Path {
                                points: p.to_vec(),
                                closed: false,
                                stroke,
                                fill: None,
                            })
                            .collect(),
                    )
                }
            }
        }
        _ => {}
    }
}

/// Add the notes layer: a filename label, the settings used, and a ruler.
///
/// The ruler exists so a printed pattern can be checked for scale.
pub fn draw_notes(scene: &mut Scene, input: &EllipsoidInput) {
    let ppu = input.px_per_unit();
    let units = input.unit.suffix();

    let canvas_height = scene
        .layer(LAYER_BOUNDING_BOX)
        .map(|l| l.bounds().height())
        .unwrap_or(scene.size.y);
    let pattern_bounds = scene
        .layer(LAYER_PATTERN)
        .map(|l| l.bounds())
        .unwrap_or(Bounds::EMPTY);

    let mut notes = Layer::new(LAYER_NOTES);

    // -- Filename, rotated to run up the left margin ----------------------
    //
    // The original rotated about the text's bottom-right corner, then set
    // `position` (paper's bounds *centre*). Placing a centre-anchored,
    // rotated run is equivalent and avoids reproducing paper's bounds algebra.
    let filename = format!("{}.svg", input.filename_stem());
    let filename_size = 0.25 * ppu;
    let filename_len = approx_text_width(&filename, filename_size);
    notes.items.push(Item::Text(Text {
        position: DVec2::new(0.15 * ppu, canvas_height - filename_len / 2.0 - 0.15 * ppu),
        content: filename,
        font_family: "Roboto".into(),
        font_size: filename_size,
        fill: Color::BLACK,
        anchor: TextAnchor::Center,
        rotation_deg: -90.0,
    }));

    // -- The settings that produced this pattern --------------------------
    //
    // The original dumped the raw Redux state. Field names differ now
    // (`h_top` rather than `hTop`, a `unit` enum rather than `ppu`), so this is
    // not byte-identical to the original stamp — but it is the same provenance
    // record, and it round-trips back into `--config`.
    //
    // Those longer names matter: the export's canvas is the content bounds, as
    // paper.js's `bounds: 'content'` gave, so a stamp wider than the pattern
    // would silently pad the document. Shrink the type to fit rather than let
    // the label dictate the page size.
    // Cutout coordinates are replaced by a count: a page-long stamp is no use
    // to someone holding a printout, and "Save settings" is the round-trip
    // path. Everything else still round-trips into `--config`.
    let mut stamped = input.clone();
    let cutout_count = std::mem::take(&mut stamped.cutouts).len();
    let mut settings = serde_json::to_string(&stamped).unwrap_or_else(|_| "{}".into());
    if cutout_count > 0 && settings.ends_with('}') {
        settings.pop();
        settings.push_str(&format!(",\"cutouts\":{cutout_count}}}"));
    }

    let margin = 0.1 * ppu;
    let canvas_width = scene
        .layer(LAYER_BOUNDING_BOX)
        .map(|l| l.bounds().width())
        .unwrap_or(scene.size.x);
    let nominal = 0.2 * ppu;
    let settings_size = if approx_text_width(&settings, nominal) > canvas_width - 2.0 * margin {
        (canvas_width - 2.0 * margin) / approx_text_width(&settings, 1.0)
    } else {
        nominal
    };
    notes.items.push(Item::Text(Text {
        position: DVec2::new(margin, 0.15 * ppu),
        content: settings,
        font_family: "Courier New".into(),
        font_size: settings_size,
        fill: Color::BLACK,
        anchor: TextAnchor::BaselineStart,
        rotation_deg: 0.0,
    }));

    // -- Ruler along the bottom -------------------------------------------
    let tick_stroke = Some(Stroke {
        color: Color::rgb(0.7, 0.3, 0.5),
        width: 0.01 * ppu,
    });
    let tick_x = pattern_bounds.min.x;
    let tick = |x: f64| Item::Path {
        points: vec![
            DVec2::new(x, canvas_height),
            DVec2::new(x, canvas_height - 0.3 * ppu),
        ],
        closed: false,
        stroke: tick_stroke,
        fill: None,
    };

    // The original added the template tick to the layer and *then* cloned it
    // once per label, so index 0 gets two coincident ticks. Preserved.
    notes.items.push(tick(tick_x));

    let tick_count = (pattern_bounds.width() / ppu).max(0.0) as usize;
    for i in 0..tick_count {
        let x = tick_x + i as f64 * ppu;
        notes.items.push(tick(x));
        notes.items.push(Item::Text(Text {
            // Label sits at the tick's centre, as paper's `position` gave.
            position: DVec2::new(x, canvas_height - 0.15 * ppu),
            content: format!("{i}{units}"),
            font_family: "Roboto".into(),
            font_size: 0.2 * ppu,
            fill: Color::BLACK,
            anchor: TextAnchor::BaselineStart,
            rotation_deg: 0.0,
        }));
    }

    scene.layers.push(notes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipsoid_core::{compute_flat_geometry, compute_geometry};

    /// A round trip through the page and back must land where it started, or
    /// dragging a cutout would move it somewhere other than the pointer.
    #[test]
    fn unplace_outline_inverts_place_outline() {
        for projection in [Projection::Spherical, Projection::Cylindrical] {
            let input = EllipsoidInput {
                projection,
                ..Default::default()
            };
            let geometry = compute_geometry(&input);
            let flat = compute_flat_geometry(&geometry, &input);
            let t = PatternTransform::new(&input, &flat);

            for p in [
                DVec3::ZERO,
                DVec3::new(3.0, -2.0, 0.0),
                DVec3::new(-7.5, 11.25, 0.0),
            ] {
                let back = t.unplace_outline(t.place_outline(p));
                assert!(
                    (back.x - p.x).abs() < 1e-9 && (back.y - p.y).abs() < 1e-9,
                    "{projection:?}: {p} -> {back}"
                );
            }
        }
    }
}

#[cfg(test)]
mod cutout_placement {
    use super::*;
    use ellipsoid_core::surface::{Cutout, SurfaceParam, flat_point};
    use ellipsoid_core::{compute_flat_geometry, compute_geometry};

    /// A cutout is drawn where the guide lines say its surface point is.
    ///
    /// Plan §8.7. The spherical cut outline was placed with `shift.y + y` while
    /// the guide lines, both quadrilateral layers and the ruler all used
    /// `shift.y - y`. Cutouts follow the outline, so a hole came out at the
    /// vertical mirror of its true position — on a star of congruent petals
    /// that looks entirely plausible and cuts the hole on the wrong petal.
    ///
    /// Checked through the whole pipeline rather than on the transform, because
    /// `place_outline == place` is now true by construction and would pass
    /// whatever the rest of the layout did.
    #[test]
    fn a_cutout_lands_where_the_guides_put_its_surface_point() {
        let (u, v) = (0.0625, 0.25);

        for projection in [Projection::Spherical, Projection::Cylindrical] {
            let input = EllipsoidInput {
                projection,
                theta_max: 90.0,
                h_middle: 0.0,
                h_bottom: 0.0,
                cutouts: vec![Cutout::hole(u, v, 0.35)],
                ..Default::default()
            };
            let geometry = compute_geometry(&input);
            let flat = compute_flat_geometry(&geometry, &input);
            let param = SurfaceParam::new(&geometry);
            let transform = PatternTransform::new(&input, &flat);
            let scene = build_scene(&input, &geometry, &flat);

            let layer = scene
                .layer(LAYER_CUTOUTS)
                .unwrap_or_else(|| panic!("{projection:?}: no cutouts layer"));
            let Some(Item::Path { points, .. }) = layer.items.first() else {
                panic!("{projection:?}: hole did not become a ring");
            };
            let centre = points.iter().fold(DVec2::ZERO, |a, p| a + *p) / points.len() as f64;

            let want = transform.place(flat_point(&param, &flat, u, v));
            // The ring is fitted around the centre, so its centroid sits within
            // a fraction of the hole's own radius of it.
            let tolerance = 0.35 * transform.ppu * 0.1;
            assert!(
                centre.distance(want) < tolerance,
                "{projection:?}: hole drawn at {centre:?}, guides put it at {want:?} \
                 (mirror would be y={})",
                2.0 * transform.shift.y * transform.ppu - want.y
            );
        }
    }
}
