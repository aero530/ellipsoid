//! On-screen preview of the flat pattern.
//!
//! Draws the [`Scene`] directly with egui's painter rather than rasterising the
//! exported SVG. The geometry is already in hand, and vector strokes stay crisp
//! at any zoom.
//!
//! **Stroke-only** (plan §7.2): the cut outline is a large concave, sometimes
//! self-intersecting polygon, and egui cannot fill those without tessellation.
//! For a pattern destined for a cutter, the lines *are* the content.

use bevy_egui::egui;
// Re-exported by core so every crate shares one vector type.
use ellipsoid_core::surface::{Cutout, SurfaceParam, flat_point, flat_to_surface};
use ellipsoid_core::{DVec2, DVec3};
use ellipsoid_pattern::{Bounds, Color, Item, PatternTransform, Scene, TextAnchor};

use crate::state::{AppState, Grab, PreviewView, Status};

/// Leave a little air around a fitted pattern.
const FIT_MARGIN: f32 = 0.95;
const MIN_ZOOM: f32 = 1e-4;
const MAX_ZOOM: f32 = 1e4;

fn to_color32(c: Color) -> egui::Color32 {
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgba_unmultiplied(byte(c.r), byte(c.g), byte(c.b), byte(c.a))
}

/// Maps scene coordinates (SVG user units, y down) to screen pixels.
struct Transform {
    origin: egui::Pos2,
    scene_center: DVec2,
    zoom: f32,
}

impl Transform {
    fn apply(&self, p: DVec2) -> egui::Pos2 {
        let d = p - self.scene_center;
        egui::pos2(
            self.origin.x + d.x as f32 * self.zoom,
            // Both SVG and egui put +y downward, so no flip.
            self.origin.y + d.y as f32 * self.zoom,
        )
    }

    /// Screen pixel back to scene coordinates.
    fn invert(&self, p: egui::Pos2) -> DVec2 {
        self.scene_center
            + DVec2::new(
                ((p.x - self.origin.x) / self.zoom) as f64,
                ((p.y - self.origin.y) / self.zoom) as f64,
            )
    }
}

/// How close, in screen pixels, the pointer must be to grab a cutout.
const GRAB_PIXELS: f32 = 10.0;
/// How close a ctrl-click must be to an outline edge to insert a point on it.
///
/// Looser than [`GRAB_PIXELS`]: an edge is a long thin target, and there is
/// nothing else to hit by mistake once the pointer is off the vertices.
const EDGE_PIXELS: f32 = 16.0;
/// Radius of the handle drawn at each cutout, in screen pixels.
const HANDLE_PIXELS: f32 = 4.0;
/// Highlight for the cutout under the pointer and for a shape being drawn.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(230, 120, 40);
const HANDLE_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 140, 200);

/// Draw the pattern and let the pointer edit its cutouts.
///
/// Editing is layered onto pan and zoom rather than replacing them: a plain
/// drag on empty space still pans. The modifiers match the 3D views, so there
/// is one set of gestures to learn.
///
/// Normally the handles are the cutouts themselves:
///
/// - **ctrl-click** empty space adds a hole (or, while drawing, adds a vertex)
/// - **shift-click** a cutout removes it
/// - **drag** a cutout moves it
/// - **double-click** a shape edits its points
///
/// In point-editing mode the handles become that one shape's vertices, and the
/// same three gestures apply to them: ctrl-click *on an outline edge* inserts a
/// point there, shift-click removes one, drag moves one.
pub fn show_editable(ui: &mut egui::Ui, state: &mut AppState) -> egui::Response {
    // Disjoint field borrows: the scene is read while the cutouts are written.
    let AppState {
        derived,
        view,
        input,
        drag,
        draft,
        editing,
        status,
        new_cutout_diameter,
        cutouts_dirty,
        dirty,
        ..
    } = state;
    let Some(derived) = derived.as_ref() else {
        return ui.allocate_response(ui.available_size(), egui::Sense::hover());
    };

    let param = SurfaceParam::new(&derived.geometry);
    let pattern = PatternTransform::new(input, &derived.flat);
    let on_page = |u: f64, v: f64| pattern.place_outline(flat_point(&param, &derived.flat, u, v));

    // An index that outlived the shape it pointed at would edit the wrong one.
    if editing.is_some_and(|i| !matches!(input.cutouts.get(i), Some(Cutout::Polygon { .. }))) {
        *editing = None;
    }

    // What the pointer can grab. `draw` needs these before it decides whether a
    // drag pans, and they do not depend on the view, so they are built here.
    let handles: Vec<(Grab, DVec2)> = match *editing {
        Some(index) => match &input.cutouts[index] {
            Cutout::Polygon { points } => points
                .iter()
                .enumerate()
                .map(|(k, p)| (Grab::Vertex(index, k), on_page(p[0], p[1])))
                .collect(),
            // Ruled out above, but the match still has to be total.
            Cutout::Hole { .. } => Vec::new(),
        },
        None => input
            .cutouts
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (u, v) = c.anchor();
                (Grab::Cutout(i), on_page(u, v))
            })
            .collect(),
    };
    let positions: Vec<DVec2> = handles.iter().map(|(_, p)| *p).collect();

    let (response, transform, nearest) = draw(ui, &derived.scene, view, &positions, drag.is_some());
    let Some(transform) = transform else {
        return response;
    };
    let nearest = nearest.map(|i| handles[i].0);

    // Screen position to a surface coordinate, or None if off the pattern.
    let surface_at = |p: egui::Pos2| -> Option<(f64, f64)> {
        let flat = pattern.unplace_outline(transform.invert(p));
        flat_to_surface(&param, &derived.flat, DVec3::new(flat.x, flat.y, 0.0))
    };

    let modifiers = ui.input(|i| i.modifiers);
    let pointer = response.interact_pointer_pos().or(response.hover_pos());
    let mut changed = false;

    // --- dragging ---------------------------------------------------------
    if response.drag_started()
        && let (Some(grab), Some(p)) = (nearest, pointer)
        && let Some((u, v)) = surface_at(p)
    {
        *drag = Some((grab, u, v));
    }
    if let Some((grab, gu, gv)) = *drag {
        if response.dragged()
            && let Some(p) = response.interact_pointer_pos()
            && let Some((u, v)) = surface_at(p)
        {
            match grab {
                Grab::Cutout(index) => {
                    if let Some(cutout) = input.cutouts.get_mut(index) {
                        cutout.translate(u - gu, v - gv);
                        changed = true;
                    }
                }
                Grab::Vertex(index, vertex) => {
                    if let Some(cutout) = input.cutouts.get_mut(index)
                        && let Cutout::Polygon { points } = cutout
                        && let Some(point) = points.get_mut(vertex)
                    {
                        // Not wrapped here: a vertex pushed past the seam has to
                        // stay on the same side of its neighbours or the shape
                        // tears. `renormalise_wrap` puts the whole thing back in
                        // range afterwards, by whole turns.
                        point[0] += u - gu;
                        point[1] = (point[1] + v - gv).clamp(0.0, 1.0);
                        cutout.renormalise_wrap();
                        changed = true;
                    }
                }
            }
            // Re-anchor every frame so the shape tracks the pointer instead of
            // drifting: the moves clamp, and a grab point that stayed put would
            // keep re-applying the rejected part of the delta.
            *drag = Some((grab, u, v));
        }
        if response.drag_stopped() {
            *drag = None;
            *status = Some(Status::info(match grab {
                Grab::Cutout(_) => "Moved cutout",
                Grab::Vertex(..) => "Moved point",
            }));
        }
    }

    // --- clicks -----------------------------------------------------------
    //
    // Double-click first: egui reports it alongside the plain click, and
    // entering point editing must not also drop a vertex or a hole.
    if response.double_clicked() {
        match nearest {
            Some(Grab::Cutout(index)) => match input.cutouts[index] {
                Cutout::Polygon { .. } => {
                    *editing = Some(index);
                    *status = Some(Status::info("Editing shape points"));
                }
                Cutout::Hole { .. } => {
                    *status = Some(Status::info("A hole has no points to edit"));
                }
            },
            // A second click on a point is not a request to leave.
            Some(Grab::Vertex(..)) => {}
            // Double-clicking away from the shape is the natural way out.
            None if editing.is_some() => {
                *editing = None;
                *status = Some(Status::info("Finished editing"));
            }
            _ => {}
        }
    } else if response.clicked()
        && let Some(p) = pointer
    {
        if modifiers.shift {
            match nearest {
                Some(Grab::Cutout(index)) => {
                    let what = input.cutouts[index].describe();
                    crate::state::forget_cutout(&mut input.cutouts, editing, drag, index);
                    *status = Some(Status::info(format!("Removed {what}")));
                    changed = true;
                }
                Some(Grab::Vertex(index, vertex)) => {
                    if let Some(Cutout::Polygon { points }) = input.cutouts.get_mut(index) {
                        if points.len() > 3 {
                            points.remove(vertex);
                            *status = Some(Status::info("Removed point"));
                            changed = true;
                        } else {
                            *status = Some(Status::error("A shape needs at least 3 points"));
                        }
                    }
                }
                None => {}
            }
        } else if let Some(index) = *editing {
            // Insert on the outline the pointer is actually pointing at, which
            // is a screen-space question — the same edge covers wildly
            // different spans of `(u, v)` at different points on the pattern.
            if modifiers.ctrl || modifiers.command {
                match insert_at(&positions, &transform, p)
                    .and_then(|(k, on_edge)| surface_at(on_edge).map(|uv| (k, uv)))
                {
                    Some((k, (u, v))) => {
                        if let Some(Cutout::Polygon { points }) = input.cutouts.get_mut(index) {
                            points.insert(k, [u, v]);
                            *status = Some(Status::info("Added point"));
                            changed = true;
                        }
                    }
                    None => {
                        *status = Some(Status::info("Ctrl-click an outline edge to add a point"))
                    }
                }
            }
        } else if let Some((u, v)) = surface_at(p) {
            match draft.as_mut() {
                // While drawing a shape, a plain click places the next vertex.
                Some(points) => {
                    points.push([u, v]);
                    *status = Some(Status::info(format!("{} points placed", points.len())));
                }
                None if modifiers.ctrl || modifiers.command => {
                    input.cutouts.push(Cutout::hole(u, v, *new_cutout_diameter));
                    *status = Some(Status::info("Added hole"));
                    changed = true;
                }
                None => {}
            }
        }
    }

    // --- handles and the shape being drawn --------------------------------
    //
    // Every cutout gets a handle, not just the ones with a visible ring: a
    // cutout that has been notched away at the outline has no ring left to aim
    // at, and would otherwise be impossible to grab back.
    let painter = ui.painter_at(response.rect.intersect(ui.clip_rect()));
    let screen: Vec<egui::Pos2> = positions.iter().map(|p| transform.apply(*p)).collect();

    if editing.is_some() && screen.len() > 1 {
        // The shape's own outline may be hidden under a notch, so draw the
        // polygon the points describe rather than relying on the pattern.
        let mut outline = screen.clone();
        outline.push(screen[0]);
        painter.add(egui::Shape::line(outline, egui::Stroke::new(1.5, ACCENT)));
    }

    for ((grab, _), at) in handles.iter().zip(&screen) {
        let active = nearest == Some(*grab) || drag.map(|(g, ..)| g) == Some(*grab);
        painter.circle_filled(
            *at,
            if active {
                HANDLE_PIXELS + 2.0
            } else {
                HANDLE_PIXELS
            },
            if active { ACCENT } else { HANDLE_COLOR },
        );
    }

    if let Some(points) = draft.as_ref()
        && !points.is_empty()
    {
        let screen: Vec<egui::Pos2> = points
            .iter()
            .map(|p| transform.apply(on_page(p[0], p[1])))
            .collect();
        if screen.len() > 1 {
            // Drawn closed, so the draft reads as the outline it will become.
            let mut outline = screen.clone();
            outline.push(screen[0]);
            painter.add(egui::Shape::line(outline, egui::Stroke::new(1.5, ACCENT)));
        }
        for p in &screen {
            painter.circle_filled(*p, HANDLE_PIXELS, ACCENT);
        }
    }

    if nearest.is_some() || drag.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    if changed {
        *cutouts_dirty = true;
        *dirty = true;
    }
    response
}

/// Where a new vertex should go, given a click near a closed outline.
///
/// Returns the index to insert *before* and the point on the edge itself, so
/// the new vertex lands on the line rather than under the cursor.
fn insert_at(
    outline: &[DVec2],
    transform: &Transform,
    pointer: egui::Pos2,
) -> Option<(usize, egui::Pos2)> {
    let screen: Vec<egui::Pos2> = outline.iter().map(|p| transform.apply(*p)).collect();
    if screen.len() < 2 {
        return None;
    }

    let mut best: Option<(f32, usize, egui::Pos2)> = None;
    for i in 0..screen.len() {
        let (a, b) = (screen[i], screen[(i + 1) % screen.len()]);
        let edge = b - a;
        let length2 = edge.length_sq();
        // A zero-length edge has no nearest point; its endpoints cover it.
        let closest = if length2 <= f32::EPSILON {
            a
        } else {
            a + edge * ((pointer - a).dot(edge) / length2).clamp(0.0, 1.0)
        };
        let distance = closest.distance(pointer);
        if best.is_none_or(|(d, ..)| distance < d) {
            best = Some((distance, i + 1, closest));
        }
    }

    best.filter(|(d, ..)| *d <= EDGE_PIXELS)
        .map(|(_, at, point)| (at, point))
}

/// Draw the scene and resolve the pointer against `grabbable`.
///
/// Hit testing happens in here rather than in the caller because the pan has to
/// be suppressed on the *same* frame a drag grabs something — panning first and
/// undoing it afterwards would jitter the page by the drag delta every frame.
///
/// Returns the transform used for the final draw, or `None` if there was
/// nothing to draw, plus the index in `grabbable` nearest the pointer.
fn draw(
    ui: &mut egui::Ui,
    scene: &Scene,
    view: &mut PreviewView,
    grabbable: &[DVec2],
    holding: bool,
) -> (egui::Response, Option<Transform>, Option<usize>) {
    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    // `available_size` can overshoot the visible region by a few pixels, which
    // is enough to clip the bottom of a freshly-fitted pattern. Fit and centre
    // against the intersection instead.
    let rect = response.rect.intersect(ui.clip_rect());

    let content = scene
        .layers
        .iter()
        .fold(Bounds::EMPTY, |acc, l| acc.union(l.bounds()));
    if content.is_empty() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return (response, None, None);
    }

    // Auto-fit until the user takes control. See `PreviewView::user_adjusted`.
    if !view.user_adjusted {
        let sx = rect.width() / content.width().max(1e-9) as f32;
        let sy = rect.height() / content.height().max(1e-9) as f32;
        view.zoom = (sx.min(sy) * FIT_MARGIN).clamp(MIN_ZOOM, MAX_ZOOM);
        view.pan = bevy::math::Vec2::ZERO;
    }

    let mut transform = Transform {
        origin: rect.center() + egui::vec2(view.pan.x, view.pan.y),
        scene_center: content.center(),
        zoom: view.zoom,
    };

    // Resolve against the pre-pan transform: that is where the things on screen
    // were when the pointer went down.
    let nearest = response
        .interact_pointer_pos()
        .or(response.hover_pos())
        .and_then(|p| {
            grabbable
                .iter()
                .map(|a| transform.apply(*a).distance(p))
                .enumerate()
                .filter(|(_, d)| *d <= GRAB_PIXELS)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
        });

    // A drag that grabbed something moves that thing, not the page.
    if response.dragged() && !holding && !(response.drag_started() && nearest.is_some()) {
        let d = response.drag_delta();
        view.pan += bevy::math::Vec2::new(d.x, d.y);
        view.user_adjusted = true;
        transform.origin = rect.center() + egui::vec2(view.pan.x, view.pan.y);
    }

    // Zoom about the cursor so the point under it stays put.
    if let Some(cursor) = response.hover_pos() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > f32::EPSILON {
            let anchor = transform.invert(cursor);
            let zoom = (view.zoom * (scroll * 0.002).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
            let d = anchor - content.center();
            view.pan = bevy::math::Vec2::new(
                cursor.x - rect.center().x - d.x as f32 * zoom,
                cursor.y - rect.center().y - d.y as f32 * zoom,
            );
            view.zoom = zoom;
            view.user_adjusted = true;
            transform.origin = rect.center() + egui::vec2(view.pan.x, view.pan.y);
            transform.zoom = zoom;
        }
    }

    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(250));
    let clip = painter.with_clip_rect(rect);
    for layer in &scene.layers {
        for item in &layer.items {
            draw_item(&clip, item, &transform);
        }
    }

    (response, Some(transform), nearest)
}

fn draw_item(painter: &egui::Painter, item: &Item, transform: &Transform) {
    match item {
        Item::Path {
            points,
            closed,
            stroke,
            ..
        } => {
            let Some(stroke) = stroke else { return };
            if points.len() < 2 {
                return;
            }
            let mut screen: Vec<egui::Pos2> = points.iter().map(|p| transform.apply(*p)).collect();
            if *closed {
                screen.push(screen[0]);
            }
            painter.add(egui::Shape::line(screen, line_stroke(*stroke, transform)));
        }
        Item::Rect { bounds, stroke, .. } => {
            let Some(stroke) = stroke else { return };
            let r =
                egui::Rect::from_two_pos(transform.apply(bounds.min), transform.apply(bounds.max));
            painter.rect_stroke(
                r,
                0.0,
                line_stroke(*stroke, transform),
                egui::StrokeKind::Middle,
            );
        }
        Item::Text(text) => {
            let size = glyph_size(text.font_size as f32 * transform.zoom);
            if !worth_rasterising(size) {
                return;
            }
            // Off-screen text still costs a full rasterisation, and the notes
            // sit at the canvas edges — exactly where they leave the view as
            // soon as anyone zooms in. `layout_no_wrap` is the expensive call,
            // so cull before it rather than leaving it to the clip rect.
            let b = item.bounds();
            let on_screen =
                egui::Rect::from_two_pos(transform.apply(b.min), transform.apply(b.max));
            if !painter.clip_rect().intersects(on_screen) {
                return;
            }
            let font = if text.font_family.eq_ignore_ascii_case("Courier New") {
                egui::FontId::monospace(size)
            } else {
                egui::FontId::proportional(size)
            };
            let color = to_color32(text.fill);
            let galley = painter.layout_no_wrap(text.content.clone(), font, color);

            let anchor = transform.apply(text.position);
            let angle = (text.rotation_deg as f32).to_radians();
            // TextShape positions the galley's top-left corner and rotates about
            // it, so shift by the half-extent the caller actually meant.
            let half = galley.size() / 2.0;
            let offset = match text.anchor {
                TextAnchor::Center => egui::vec2(-half.x, -half.y),
                TextAnchor::BaselineStart => egui::vec2(0.0, -galley.size().y * 0.8),
            };
            let rotated = egui::vec2(
                offset.x * angle.cos() - offset.y * angle.sin(),
                offset.x * angle.sin() + offset.y * angle.cos(),
            );
            painter.add(
                egui::epaint::TextShape::new(anchor + rotated, galley, color).with_angle(angle),
            );
        }
        Item::Group(children) => {
            for child in children {
                draw_item(painter, child, transform);
            }
        }
    }
}

/// Largest glyph height, in screen pixels, that is worth asking egui for.
///
/// The real constraint is egui's font atlas: glyphs are packed into a texture
/// at most 2048 wide, and `epaint` *panics* rather than declining when one will
/// not fit. Scaling font size by the view's zoom meant the ruler labels — 0.2
/// of a unit, so ~19 px at 1:1 — blew past that at about 100× and took the
/// whole app down. This is far below the atlas limit and already far above
/// anything legible on screen.
const MAX_GLYPH_PX: f32 = 256.0;

/// Snap a font size to a step, so zooming cannot mint unbounded font instances.
///
/// `epaint` caches rasterised glyphs per `FontId`, and a `FontId` carries the
/// size as an exact `f32`. Scaling text by a smoothly changing zoom therefore
/// asks for a *new* font on almost every frame, each one rasterising its own
/// copy of every glyph drawn — which fills the atlas and gets as far as
/// `epaint texture atlas overflowed!`.
///
/// Rounding to whole pixels below 32 and to multiples of 8 above caps the whole
/// range at about fifty distinct sizes. The visible difference between adjacent
/// steps while zooming is nothing; the difference in glyph churn is the bug.
fn glyph_size(size: f32) -> f32 {
    if size < 32.0 {
        size.round()
    } else {
        (size / 8.0).round() * 8.0
    }
}

/// Whether text at this on-screen size should be drawn at all.
///
/// Below a few pixels the glyphs are noise. Above [`MAX_GLYPH_PX`] they cannot
/// be rasterised, and clamping instead of skipping would be worse than either:
/// the notes are a *ruler*, so a label held at a size the drawing has outgrown
/// would misreport scale, which is the one thing it exists to convey.
fn worth_rasterising(size: f32) -> bool {
    (4.0..=MAX_GLYPH_PX).contains(&size)
}

/// Scale stroke width with the view, but never let a line vanish.
fn line_stroke(stroke: ellipsoid_pattern::Stroke, transform: &Transform) -> egui::Stroke {
    egui::Stroke {
        width: (stroke.width as f32 * transform.zoom).max(0.75),
        color: to_color32(stroke.color),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ruler label size the notes layer uses, at 96 px/in.
    const RULER_PX: f32 = 0.2 * 96.0;

    /// Zooming smoothly must not mint a new font size per frame — that is what
    /// overflowed the glyph atlas.
    #[test]
    fn zooming_asks_for_only_a_handful_of_sizes() {
        let sizes: std::collections::BTreeSet<u32> = (0..20_000)
            .map(|i| {
                let zoom = MIN_ZOOM * (MAX_ZOOM / MIN_ZOOM).powf(i as f32 / 20_000.0);
                glyph_size(RULER_PX * zoom)
            })
            .filter(|s| worth_rasterising(*s))
            .map(|s| s.to_bits())
            .collect();

        assert!(
            sizes.len() <= 64,
            "{} distinct font sizes across the zoom range",
            sizes.len()
        );
        // And it must not collapse to so few that text visibly jumps.
        assert!(sizes.len() >= 16, "only {} sizes; too coarse", sizes.len());
    }

    #[test]
    fn quantising_stays_close_to_the_size_asked_for() {
        for size in [4.0_f32, 7.3, 12.9, 31.7, 40.0, 100.0, 255.0] {
            let snapped = glyph_size(size);
            let step = if size < 32.0 { 0.5 } else { 4.0 };
            assert!(
                (snapped - size).abs() <= step,
                "{size} snapped to {snapped}"
            );
        }
    }

    #[test]
    fn deep_zoom_does_not_ask_for_an_unrasterisable_glyph() {
        // This crashed the app: `epaint` panics rather than declining when a
        // glyph will not fit its atlas, so the ceiling has to be ours.
        assert!(!worth_rasterising(RULER_PX * MAX_ZOOM));
        // The threshold is around 13x for a ruler label. Either side of it.
        assert!(worth_rasterising(RULER_PX * 10.0));
        assert!(!worth_rasterising(RULER_PX * 100.0));
    }

    #[test]
    fn text_too_small_to_read_is_still_skipped() {
        assert!(!worth_rasterising(RULER_PX * MIN_ZOOM));
        assert!(!worth_rasterising(3.9));
        assert!(worth_rasterising(4.0));
    }

    #[test]
    fn the_ceiling_clears_nothing_a_person_could_read() {
        // A cap that cut into legible sizes would be a worse bug than the
        // crash it prevents. 64 px is already a headline.
        assert!(worth_rasterising(64.0));
        assert!(worth_rasterising(MAX_GLYPH_PX));
    }
}
