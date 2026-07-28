//! Desktop and web GUI for the ellipsoid flat-pattern generator.
//!
//! One binary serves both targets: `cargo run -p ellipsoid-app` for desktop,
//! `trunk serve` for the browser.
//!
//! # Status
//!
//! Settings, a live 2D pattern preview, and two 3D views — the ellipsoid and
//! the flattened pattern — each rendered to a texture and shown inside an egui
//! panel. Cutouts can be placed by picking in the 3D views or drawn and edited
//! directly on the pattern, and are cut from both previews. Settings save,
//! load, and are remembered between sessions. See `RUST_CONVERSION_PLAN.md`.

mod cutouts;
mod mesh;
mod platform;
mod preview;
mod state;
mod ui;
mod viewport;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bevy_panorbit_camera::{PanOrbitCameraPlugin, PanOrbitCameraSystemSet};

use state::{AppState, Persistence, recompute};

fn main() {
    App::new()
        .init_resource::<AppState>()
        .init_resource::<Persistence>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ellipsoid Pattern Generator".into(),
                // Ignored on desktop; on wasm this binds to the <canvas> in index.html.
                canvas: Some("#ellipsoid-canvas".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_systems(Startup, (viewport::setup, state::restore))
        .add_systems(
            Update,
            (
                // Picks and opened files are queued by the UI in the previous
                // frame's egui pass, so resolve them before recomputing.
                cutouts::apply_picks,
                state::apply_opened,
                recompute,
                viewport::sync_meshes,
                cutouts::sync_markers,
                viewport::sync_material,
                viewport::apply_viewport_sizes,
                // Must land before the plugin reads it, and `manual: true`
                // keeps the plugin from overwriting our choice.
                viewport::route_camera_input.before(PanOrbitCameraSystemSet),
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, ui::draw)
        // In `Last` so the exit flush sees the frame's edits, and after the
        // egui pass so a change made this frame starts its idle timer from now.
        .add_systems(Last, (state::autosave, state::autosave_on_exit).chain())
        .run();
}
