//! The egui shell: toolbar, settings panel, pattern preview, status bar.
//!
//! Tooltip text is carried over verbatim from the Material-UI original — it is
//! the only real documentation these parameters ever had.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use ellipsoid_core::{Cutout, EllipsoidInput, Projection, Unit, flat_to_obj, geometry_to_obj};
use ellipsoid_pattern::{SvgOptions, to_svg};

use crate::platform::{self, Saved};
use crate::preview;
use crate::state::{AppState, Status, SurfaceMaterial};
use crate::viewport::{PendingPick, PickAction, ViewKind, Viewports};

/// Unbounded above; the original left most maxima open too.
const NO_MAX: f64 = f64::INFINITY;

pub fn draw(
    mut contexts: EguiContexts,
    mut state: ResMut<AppState>,
    mut viewports: ResMut<Viewports>,
    windows: Query<&Window>,
) -> Result {
    let window_height = windows.iter().next().map(|w| w.height()).unwrap_or(0.0);
    // Texture ids must be resolved before `ctx_mut` borrows `contexts`.
    let textures: Vec<(ViewKind, Option<egui::TextureId>)> = ViewKind::ALL
        .into_iter()
        .map(|kind| {
            let handle = viewports.get(kind).image.clone();
            (kind, contexts.image_id(&handle))
        })
        .collect();

    let ctx = contexts.ctx_mut()?.clone();

    // egui 0.35 hangs panels off a Ui covering the viewport rather than the
    // context directly; this is the shape every bevy_egui example uses.
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    // Toolbar carries the status line as a second row.
    //
    // A `Panel::bottom` would be the conventional home for it, but bottom
    // panels neither reserve space nor render inside this background-layer
    // viewport Ui — verified with `exact_size(80)` and a debug rect, while top
    // and left panels behave normally. Not worth chasing for a status line.
    let toolbar_height = egui::Panel::top("toolbar")
        .show(&mut viewport_ui, |ui| {
            toolbar(ui, &mut state);
            ui.separator();
            status_bar(ui, &state);
        })
        .response
        .rect
        .height();
    egui::Panel::left("settings")
        .default_size(320.0)
        .resizable(true)
        .show(&mut viewport_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| settings(ui, &mut state));
        });
    // A second left panel rather than a right one: like `Panel::bottom` above,
    // far-edge panels do not reserve space or render inside this
    // background-layer viewport Ui, while near-edge ones behave normally.
    // Stacking left panels gives the original's settings | 3D | pattern
    // arrangement anyway.
    egui::Panel::left("views")
        .default_size(views_width(&viewport_ui, toolbar_height, window_height))
        .resizable(true)
        .show(&mut viewport_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                views_panel(ui, &mut state, &mut viewports, &textures, window_height)
            });
        });
    egui::CentralPanel::default().show(&mut viewport_ui, |ui| preview_area(ui, &mut state));

    Ok(())
}

// ---------------------------------------------------------------------------
// 3D views
// ---------------------------------------------------------------------------

/// Smallest a view is allowed to get before the column scrolls instead.
const MIN_VIEW: f32 = 96.0;

/// Everything stacked with the two views that is not a view: a title above each
/// one, the two hint lines below, and the gaps between them all.
fn views_chrome(ui: &egui::Ui, gaps: f32) -> f32 {
    2.0 * ui.text_style_height(&egui::TextStyle::Body)
        + 2.0 * ui.text_style_height(&egui::TextStyle::Small)
        + gaps * ui.spacing().item_spacing.y
}

/// Default width of the 3D column: the one that has the two views fill the
/// window's height at 4:3 each.
///
/// Only the starting width — [`views_panel`] sizes the views from the height
/// actually available, so dragging the splitter changes their aspect rather than
/// whether they fit. Getting this right just means they begin square-ish instead
/// of letterboxed, with the flat pattern taking everything left over.
fn views_width(ui: &egui::Ui, toolbar_height: f32, window_height: f32) -> f32 {
    // Two more gaps than `views_panel` counts, and the material row: it measures
    // from a cursor already past both.
    let chrome = views_chrome(ui, 7.0) + ui.spacing().interact_size.y;
    let for_views = window_height - toolbar_height - chrome;
    (for_views / 2.0).max(MIN_VIEW) * 4.0 / 3.0
}

/// Show both off-screen renders, stacked, and record their size and hover state
/// so [`crate::viewport`] can resize the textures and route camera input.
fn views_panel(
    ui: &mut egui::Ui,
    state: &mut AppState,
    viewports: &mut Viewports,
    textures: &[(ViewKind, Option<egui::TextureId>)],
    window_height: f32,
) {
    ui.horizontal(|ui| {
        ui.label("Material");
        egui::ComboBox::from_id_salt("material")
            .selected_text(state.material.label())
            .show_ui(ui, |ui| {
                for material in SurfaceMaterial::ALL {
                    ui.selectable_value(&mut state.material, material, material.label());
                }
            })
            .response
            .on_hover_text("The UV grid puts one tile per panel cell, the same way in both views");
    });

    // Each view takes half of whatever height is really left, so the pair of
    // them fills the window instead of leaving a gap under the second one.
    //
    // Neither obvious source of that height works. `available_height` reports
    // far more space than exists inside a panel on this background-layer Ui —
    // enough that the second view once landed below the window entirely. And
    // `ctx.viewport_rect()`, which is in points on every later frame, reports
    // *physical pixels* on the first one, so on a 150% display it starts out
    // half again too tall. The window's own height is right from the first frame
    // and is in points, since bevy_egui matches egui's scale to the window's.
    //
    // `cursor` is reliable, and is exactly where the first view will go, so the
    // two together give the height that is really left. The panel still scrolls
    // if the window is too short for `MIN_VIEW`.
    let width = ui.available_width().max(64.0);
    let left = window_height - ui.cursor().top() - views_chrome(ui, 5.0);
    // A point in hand, so rounding cannot summon a scrollbar — which would take
    // width from the views for no reason.
    let each = ((left - 1.0) / 2.0).max(MIN_VIEW);
    let mut pending = None;

    for (kind, texture) in textures {
        ui.label(egui::RichText::new(kind.title()).strong());

        let size = egui::vec2(width, each);

        let response = match texture {
            Some(id) => ui.add(
                egui::Image::new(egui::load::SizedTexture::new(*id, size))
                    .sense(egui::Sense::click_and_drag()),
            ),
            None => ui.allocate_response(size, egui::Sense::hover()),
        };

        // A modified click places or removes a hole. Plain drags stay with the
        // orbit controller, so editing never fights navigation.
        let modifiers = ui.input(|i| i.modifiers);
        if response.clicked()
            && (modifiers.ctrl || modifiers.command || modifiers.shift)
            && let Some(pos) = response.interact_pointer_pos()
        {
            let rect = response.rect;
            pending = Some(PendingPick {
                view: *kind,
                normalized: Vec2::new(
                    ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0),
                    ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0),
                ),
                action: if modifiers.shift {
                    PickAction::Remove
                } else {
                    PickAction::Add
                },
            });
        }

        let view = viewports.get_mut(*kind);
        // Textures are sized in physical pixels; egui lays out in points.
        let ppp = ui.ctx().pixels_per_point();
        view.desired = UVec2::new((size.x * ppp).round() as u32, (size.y * ppp).round() as u32);
        view.hovered = response.contains_pointer();
    }

    if pending.is_some() {
        viewports.pending_pick = pending;
    }

    ui.label(
        egui::RichText::new("drag to orbit, scroll to zoom")
            .small()
            .weak(),
    );
    ui.label(
        egui::RichText::new("ctrl-click the ellipsoid to add a hole, shift-click to remove")
            .small()
            .weak(),
    );
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

fn toolbar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        let ready = state.derived.is_some();

        if ui
            .add_enabled(ready, egui::Button::new("Save SVG"))
            .on_hover_text("Export the flat pattern for cutting or printing")
            .clicked()
        {
            save_svg(state);
        }
        if ui
            .add_enabled(ready, egui::Button::new("Save OBJ"))
            .on_hover_text("Export the ellipsoid surface as a 3D mesh")
            .clicked()
        {
            save_obj(state, false);
        }
        if ui
            .add_enabled(ready, egui::Button::new("Save flat OBJ"))
            .on_hover_text("Export the flattened pattern as a 3D mesh")
            .clicked()
        {
            save_obj(state, true);
        }

        ui.separator();

        if ui
            .add_enabled(ready, egui::Button::new("Save settings"))
            .on_hover_text("Write the current parameters as JSON")
            .clicked()
        {
            save_settings(state);
        }
        if ui
            .button("Load settings")
            .on_hover_text("Read parameters back from a JSON file")
            .clicked()
        {
            // The answer arrives through the inbox, immediately on desktop and
            // whenever the browser's picker resolves on the web.
            platform::open_json(&state.inbox);
        }

        ui.separator();

        if ui
            .button("Reset")
            .on_hover_text("Restore the default parameters")
            .clicked()
        {
            state.input = EllipsoidInput::default();
            state.view.user_adjusted = false;
            state.status = Some(Status::info("Reset to defaults"));
            state.touch();
        }
    });
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// One labelled number field. Returns whether it changed.
fn number(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
    value: &mut f64,
    step: f64,
    range: RangeInclusive<f64>,
) -> bool {
    let mut changed = false;
    ui.label(label).on_hover_text(tooltip);
    changed |= ui
        .add(egui::DragValue::new(value).speed(step).range(range))
        .on_hover_text(tooltip)
        .changed();
    ui.end_row();
    changed
}

fn count(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
    value: &mut usize,
    range: RangeInclusive<usize>,
) -> bool {
    let mut changed = false;
    ui.label(label).on_hover_text(tooltip);
    changed |= ui
        .add(egui::DragValue::new(value).speed(1.0).range(range))
        .on_hover_text(tooltip)
        .changed();
    ui.end_row();
    changed
}

fn settings(ui: &mut egui::Ui, state: &mut AppState) {
    // Edit a copy so the change-detection flag is only set on a real edit.
    // Cutouts are edited in place further down, so this copy leaves them out.
    let mut input = EllipsoidInput {
        cutouts: Vec::new(),
        ..state.input.clone()
    };
    let mut changed = false;

    ui.add_space(4.0);
    ui.heading("Geometry");
    egui::Grid::new("geometry")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            // Steps and limits mirror the original's inputProps.
            changed |= number(ui, "a", "semi axis a", &mut input.a, 0.125, 0.125..=NO_MAX);
            changed |= number(ui, "b", "semi axis b", &mut input.b, 0.125, 0.125..=NO_MAX);
            changed |= number(ui, "c", "semi axis c", &mut input.c, 0.125, 0.125..=NO_MAX);

            changed |= number(
                ui,
                "hTop",
                "added height at top of open ellipsoid (theta max < 90)",
                &mut input.h_top,
                0.125,
                0.0..=NO_MAX,
            );
            changed |= number(
                ui,
                "hMiddle",
                "added thickness in the middle of the ellipsoid (vertically)",
                &mut input.h_middle,
                0.125,
                0.0..=NO_MAX,
            );
            changed |= number(
                ui,
                "hBottom",
                "added height at the bottom of an open ellipsoid (theta min > -90)",
                &mut input.h_bottom,
                0.125,
                0.0..=NO_MAX,
            );
            changed |= number(
                ui,
                "hTopFraction",
                "scaling factor put on the hTop ellipse (based on the ellipse at thetaMax)",
                &mut input.h_top_fraction,
                0.125,
                0.125..=2.0,
            );
            changed |= number(
                ui,
                "hTopShift",
                "factor used to shift the hTop ellipse side to side",
                &mut input.h_top_shift,
                0.125,
                -5.0..=5.0,
            );

            changed |= number(
                ui,
                "thetaMin",
                "Angle defining the bottom of the ellipsoid.  -90 is fully closed on the bottom",
                &mut input.theta_min,
                1.0,
                -90.0..=85.0,
            );
            changed |= number(
                ui,
                "thetaMax",
                "Angle defining the top of the ellipsoid.  90 is fully closed on the top",
                &mut input.theta_max,
                1.0,
                -85.0..=90.0,
            );

            changed |= count(
                ui,
                "Divisions",
                "Number of longitudinal divisions of the ellipsoid.",
                &mut input.phi_divisions,
                3..=100,
            );
            changed |= count(
                ui,
                "divisions",
                "Number of latitudinal divisions of the ellipsoid.",
                &mut input.theta_divisions,
                3..=100,
            );
        });

    ui.add_space(8.0);
    ui.heading("Projection");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Units");
        egui::ComboBox::from_id_salt("units")
            .selected_text(input.unit.to_string())
            .show_ui(ui, |ui| {
                for unit in Unit::ALL {
                    changed |= ui
                        .selectable_value(&mut input.unit, unit, unit.suffix())
                        .changed();
                }
            });
    });

    ui.add_space(4.0);
    ui.label("Projection Type").on_hover_text(
        "Pattern projection type.  Spherical = Unfold from the top of the ellipsoid.  \
         Cylindrical = unfold from the front of the ellipsoid.",
    );
    for projection in Projection::ALL {
        changed |= ui
            .radio_value(&mut input.projection, projection, projection.name())
            .changed();
    }

    ui.add_space(4.0);
    egui::Grid::new("projection")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            changed |= number(
                ui,
                "imageOffset",
                "Padding the SVG around the ellipsoid pattern",
                &mut input.image_offset,
                0.25,
                0.0..=NO_MAX,
            );
            changed |= number(
                ui,
                "minGap",
                "Minimum gap allowed between lines in the SVG image.  \
                 Helpful for allowing for cutting tool radius.",
                &mut input.min_gap,
                0.001,
                0.0..=NO_MAX,
            );
        });

    changed |= ui
        .checkbox(&mut input.inkscape_layers, "Inkscape Layers")
        .on_hover_text("Save SVG as Inkscape file with layers.  Turn off for plain SVG.")
        .changed();

    if changed {
        input.cutouts = std::mem::take(&mut state.input.cutouts);
        state.input = input;
        state.touch();
    }

    cutouts_section(ui, state);

    if !state.problems.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), "Invalid parameters");
        for problem in &state.problems {
            ui.colored_label(egui::Color32::from_rgb(200, 60, 60), format!("• {problem}"));
        }
    }
}

fn cutouts_section(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(8.0);
    ui.heading("Cutouts");
    ui.add_space(4.0);

    let units = state.input.unit.suffix();

    ui.horizontal(|ui| {
        ui.label("New hole ⌀").on_hover_text(
            "Diameter given to the next hole placed. Existing holes keep their own size.",
        );
        ui.add(
            egui::DragValue::new(&mut state.new_cutout_diameter)
                .speed(0.01)
                .range(0.001..=NO_MAX)
                .suffix(units),
        );
    });

    if state.input.cutouts.is_empty() {
        ui.label(
            egui::RichText::new("ctrl-click the ellipsoid view to add one")
                .small()
                .weak(),
        );
        return;
    }

    // How many cut pieces each cutout ends up as. More than one means it sits
    // on a seam and is divided between panels — information, not a problem.
    let piece_counts: Vec<usize> = match &state.derived {
        Some(derived) => {
            let param = ellipsoid_core::SurfaceParam::new(&derived.geometry);
            state
                .input
                .cutouts
                .iter()
                .map(|c| {
                    ellipsoid_pattern::cutouts::pieces(&param, &derived.geometry, &derived.flat, c)
                        .len()
                })
                .collect()
        }
        None => vec![1; state.input.cutouts.len()],
    };

    let mut remove = None;
    let mut changed = false;

    egui::Grid::new("cutouts")
        .num_columns(3)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (i, cutout) in state.input.cutouts.iter_mut().enumerate() {
                let split = piece_counts.get(i).copied().unwrap_or(1) > 1;
                let label = if split {
                    egui::RichText::new(format!("{}⧗", i + 1)).weak()
                } else {
                    egui::RichText::new(format!("{}", i + 1))
                };
                let response = ui.label(label);
                if split {
                    response.on_hover_text(format!(
                        "Sits on a seam, so it is cut as {} pieces — one per panel. \
                         The panel edge is opened up between them, so they form the \
                         shape once the panels are joined.",
                        piece_counts[i]
                    ));
                }

                match cutout {
                    ellipsoid_core::Cutout::Hole { diameter, .. } => {
                        changed |= ui
                            .add(
                                egui::DragValue::new(diameter)
                                    .speed(0.01)
                                    .range(0.001..=NO_MAX)
                                    .suffix(units),
                            )
                            .changed();
                    }
                    ellipsoid_core::Cutout::Polygon { points } => {
                        ui.label(
                            egui::RichText::new(format!("shape, {} pts", points.len())).weak(),
                        );
                    }
                }

                if ui.small_button("✕").on_hover_text("Delete").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });

    if ui.button("Clear all").clicked() {
        state.input.cutouts.clear();
        // Nothing left to be editing or dragging.
        state.editing = None;
        state.drag = None;
        changed = true;
    }
    if let Some(i) = remove {
        crate::state::forget_cutout(
            &mut state.input.cutouts,
            &mut state.editing,
            &mut state.drag,
            i,
        );
        changed = true;
    }
    if changed {
        state.cutouts_dirty = true;
        state.touch();
    }
}

// ---------------------------------------------------------------------------
// Preview
// ---------------------------------------------------------------------------

fn preview_area(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.heading("Flat pattern");
        if ui
            .button("Fit")
            .on_hover_text("Zoom to fit the whole pattern")
            .clicked()
        {
            state.view.user_adjusted = false;
        }
        ui.label(
            egui::RichText::new(format!("{:.0}%", state.view.zoom * 100.0))
                .small()
                .weak(),
        );

        ui.separator();

        draw_shape_controls(ui, state);
    });
    ui.separator();

    if state.derived.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label("Fix the parameters on the left to see a pattern.");
        });
        return;
    }

    preview::show_editable(ui, state);
}

/// Enter/leave the polygon drafting mode, and report progress while in it.
///
/// The keyboard shortcuts matter more than the buttons: placing points is a
/// mouse job in the middle of the canvas, and reaching back up to the toolbar
/// to close a shape breaks the rhythm.
fn draw_shape_controls(ui: &mut egui::Ui, state: &mut AppState) {
    if let Some(index) = state.editing {
        let count = match state.input.cutouts.get(index) {
            Some(Cutout::Polygon { points }) => points.len(),
            _ => 0,
        };
        if ui.button("Done").on_hover_text("Escape").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            state.editing = None;
            state.status = Some(Status::info("Finished editing"));
        }
        ui.label(
            egui::RichText::new(format!(
                "editing {count} points — drag to move · \
                 ctrl-click an edge to add · shift-click a point to remove"
            ))
            .small()
            .weak(),
        );
        return;
    }

    let Some(points) = state.draft.as_ref() else {
        if ui
            .button("Draw shape")
            .on_hover_text("Click points on the pattern to outline a cutout")
            .clicked()
        {
            state.draft = Some(Vec::new());
        }
        ui.label(
            egui::RichText::new(
                "drag to pan · scroll to zoom · ctrl-click adds a hole · \
                 drag a handle to move · shift-click removes · \
                 double-click a shape to edit its points",
            )
            .small()
            .weak(),
        );
        return;
    };

    let count = points.len();
    let enough = count >= 3;
    let (finish, cancel, undo) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::Backspace),
        )
    });

    let finish = ui
        .add_enabled(enough, egui::Button::new("Finish shape"))
        .on_hover_text("Close the outline and cut it (Enter)")
        .clicked()
        || (finish && enough);
    let cancel = ui.button("Cancel").on_hover_text("Escape").clicked() || cancel;
    let undo = ui
        .add_enabled(count > 0, egui::Button::new("Undo point"))
        .on_hover_text("Backspace")
        .clicked()
        || (undo && count > 0);

    ui.label(
        egui::RichText::new(format!(
            "click to place points — {count} placed{}",
            if enough { "" } else { ", 3 needed" }
        ))
        .small()
        .weak(),
    );

    if finish {
        let points = state.draft.take().expect("still drawing");
        state.input.cutouts.push(Cutout::polygon(points));
        state.status = Some(Status::info("Added shape"));
        state.cutouts_dirty = true;
        state.touch();
    } else if cancel {
        state.draft = None;
        state.status = Some(Status::info("Discarded shape"));
    } else if undo {
        state.draft.as_mut().expect("still drawing").pop();
    }
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn status_bar(ui: &mut egui::Ui, state: &AppState) {
    ui.horizontal(|ui| {
        match &state.status {
            Some(status) if status.is_error => {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), &status.text);
            }
            Some(status) => {
                ui.label(&status.text);
            }
            None => {
                ui.label(
                    egui::RichText::new(format!("{}.svg", state.filename_stem()))
                        .monospace()
                        .weak(),
                );
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(derived) = &state.derived {
                let size = derived.scene.size;
                let ppu = state.input.px_per_unit();
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} x {:.2} {}   {} x {} panels",
                        size.x / ppu,
                        size.y / ppu,
                        state.input.unit.suffix(),
                        derived.geometry.phi_divisions,
                        derived.geometry.theta_divisions,
                    ))
                    .weak(),
                );
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Saving
// ---------------------------------------------------------------------------

fn report(state: &mut AppState, what: &str, result: Result<Saved, String>) {
    state.status = Some(match result {
        Ok(Saved::To(path)) => Status::info(format!("Saved {what} to {path}")),
        Ok(Saved::Downloaded) => Status::info(format!("Downloaded {what}")),
        Ok(Saved::Cancelled) => Status::info("Save cancelled"),
        Err(why) => Status::error(format!("Could not save {what}: {why}")),
    });
}

fn save_svg(state: &mut AppState) {
    let Some(derived) = &state.derived else {
        return;
    };
    let svg = to_svg(
        &derived.scene,
        &SvgOptions {
            inkscape_layers: state.input.inkscape_layers,
        },
    );
    let name = format!("{}.svg", state.filename_stem());
    let result = platform::save_text(&name, &svg);
    report(state, "SVG", result);
}

fn save_obj(state: &mut AppState, flattened: bool) {
    let Some(derived) = &state.derived else {
        return;
    };
    let (obj, suffix, what) = if flattened {
        (flat_to_obj(&derived.flat), "-flat", "flat OBJ")
    } else {
        (geometry_to_obj(&derived.geometry), "", "OBJ")
    };
    let name = format!("{}{suffix}.obj", state.filename_stem());
    let result = platform::save_text(&name, &obj);
    report(state, what, result);
}

fn save_settings(state: &mut AppState) {
    let name = format!("{}.json", state.filename_stem());
    let result = platform::save_text(&name, &state.input.to_json());
    report(state, "settings", result);
}
