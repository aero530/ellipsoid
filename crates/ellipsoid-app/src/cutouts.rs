//! Placing cutouts by clicking in the 3D views, and showing where they landed.
//!
//! # Why this raycasts by hand
//!
//! `bevy_picking` works from a pointer over an on-screen camera. Both 3D views
//! render to off-screen textures that egui then draws, so a click over one of
//! those images has no pointer Bevy can associate with a ray — driving it would
//! mean registering custom pointers and keeping their `NormalizedRenderTarget`
//! in step with egui's layout.
//!
//! Intersecting the ray with the *core* geometry instead is both simpler and
//! more direct: [`ellipsoid_core::ray_hit`] returns a surface coordinate, which
//! is exactly what a cutout stores. No world-space round trip, no dependence on
//! how the Bevy mesh happens to be built.

use bevy::prelude::*;
use ellipsoid_core::surface::{SurfaceParam, flat_point, ray_hit, surface_point};
use ellipsoid_core::{Cutout, DVec3};

use crate::mesh::{from_bevy, to_bevy};
use crate::state::{AppState, Status};
use crate::viewport::{PickAction, ViewKind, Viewports};

/// Marks a cutout indicator so the whole set can be rebuilt at once.
#[derive(Component)]
pub struct CutoutMarker;

/// How close a click has to be, in surface units, to count as hitting a cutout.
///
/// A hole answers with its own size; a polygon has no single radius, so it gets
/// a fixed reach around its centre.
pub fn pick_radius(cutout: &Cutout) -> f64 {
    match cutout {
        Cutout::Hole { diameter, .. } => diameter.max(1e-6),
        Cutout::Polygon { .. } => 0.25,
    }
}

/// Build a world-space ray through a normalised point in a viewport image.
///
/// `normalized` is `(0,0)` at the image's top-left and `(1,1)` at its
/// bottom-right, matching egui's screen convention.
fn ray_through(
    transform: &GlobalTransform,
    projection: &Projection,
    normalized: Vec2,
) -> Option<(Vec3, Vec3)> {
    let Projection::Orthographic(ortho) = projection else {
        // Only the orthographic views are pickable; a perspective view would
        // need the ray built from the frustum instead.
        return None;
    };

    // `area` is the visible rect in view space, y up — hence the flip.
    let area = ortho.area;
    let x = area.min.x + normalized.x * area.width();
    let y = area.max.y - normalized.y * area.height();

    let origin = transform.transform_point(Vec3::new(x, y, 0.0));
    let direction = *transform.forward();
    Some((origin, direction))
}

/// Turn a queued click into a cutout added or removed.
pub fn apply_picks(
    mut viewports: ResMut<Viewports>,
    mut state: ResMut<AppState>,
    cameras: Query<(&GlobalTransform, &Projection)>,
) {
    let Some(pick) = viewports.pending_pick.take() else {
        return;
    };
    // Only the ellipsoid view places holes: a cutout is a position on the
    // surface, and picking the flattened copy would be ambiguous at seams.
    if pick.view != ViewKind::Surface {
        return;
    }
    let Some(derived) = &state.derived else {
        return;
    };

    let camera = viewports.get(pick.view).camera;
    let Ok((transform, projection)) = cameras.get(camera) else {
        return;
    };
    let Some((origin, direction)) = ray_through(transform, projection, pick.normalized) else {
        return;
    };

    let param = SurfaceParam::new(&derived.geometry);
    let hit = ray_hit(
        &param,
        &derived.geometry,
        from_bevy(origin),
        from_bevy(direction),
    );

    let Some(hit) = hit else {
        state.status = Some(Status::info("Clicked past the ellipsoid"));
        return;
    };

    match pick.action {
        PickAction::Add => {
            // Read the diameter before taking the mutable borrow.
            let cutout = Cutout::hole(hit.u, hit.v, state.new_cutout_diameter);
            state.input.cutouts.push(cutout);
            let n = state.input.cutouts.len();
            state.status = Some(Status::info(format!("Added hole {n}")));
            state.touch();
        }
        PickAction::Remove => {
            let target = hit.point;
            let nearest = state
                .input
                .cutouts
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let (u, v) = c.anchor();
                    let p = surface_point(&param, &derived.geometry, u, v);
                    (i, (p - target).length(), pick_radius(c))
                })
                .filter(|(_, d, reach)| d <= reach)
                .min_by(|a, b| a.1.total_cmp(&b.1));

            match nearest {
                Some((index, ..)) => {
                    let what = state.input.cutouts[index].describe();
                    let message = format!("Removed {what}");
                    state.input.cutouts.remove(index);
                    state.status = Some(Status::info(message));
                    state.touch();
                }
                None => state.status = Some(Status::info("Nothing there to remove")),
            }
        }
    }
}

/// Rebuild the marker spheres shown in both 3D views.
///
/// Cheap to redo wholesale: there are only ever a handful of cutouts, and this
/// avoids tracking which marker belongs to which index across edits.
pub fn sync_markers(
    mut commands: Commands,
    state: Res<AppState>,
    viewports: Res<Viewports>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<Entity, With<CutoutMarker>>,
    mut last: Local<Option<(u64, usize)>>,
) {
    let Some(derived) = &state.derived else {
        return;
    };

    // Rebuild only when the geometry or the cutout set actually changed.
    let signature = (state.geometry_generation, state.input.cutouts.len());
    if *last == Some(signature) && !state.cutouts_dirty {
        return;
    }
    *last = Some(signature);

    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if state.input.cutouts.is_empty() {
        return;
    }

    let param = SurfaceParam::new(&derived.geometry);
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.15, 0.15),
        unlit: true,
        ..default()
    });

    // A dot at the anchor, not a sphere the size of the hole: the hole itself is
    // now cut out of the mesh, so a marker that filled it would hide it. Sized
    // from the profile so it reads the same whatever unit the document is in.
    let radius = (param.profile_length() * 0.01).max(1e-3) as f32;
    let sphere = meshes.add(Sphere::new(radius).mesh().uv(12, 8));

    for cutout in &state.input.cutouts {
        let (u, v) = cutout.anchor();

        for view in &viewports.views {
            let position: DVec3 = match view.kind {
                ViewKind::Surface => surface_point(&param, &derived.geometry, u, v),
                ViewKind::Flat => flat_point(&param, &derived.flat, u, v),
            };
            commands.spawn((
                CutoutMarker,
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(to_bevy(position)),
                bevy::camera::visibility::RenderLayers::layer(view.kind.layer()),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::ScalingMode;
    use ellipsoid_core::surface::SurfaceParam;
    use ellipsoid_core::{EllipsoidInput, compute_geometry};

    /// An orthographic camera 20 units back along +Z, looking at the origin,
    /// showing 10 world units vertically across a square image.
    fn camera() -> (GlobalTransform, Projection) {
        let transform = GlobalTransform::from(
            Transform::from_xyz(0.0, 0.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        );
        let mut ortho = OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 10.0,
            },
            ..OrthographicProjection::default_3d()
        };
        // `area` is normally filled in by Bevy from the render target.
        ortho.area = Rect::from_center_size(Vec2::ZERO, Vec2::splat(10.0));
        (transform, Projection::Orthographic(ortho))
    }

    #[test]
    fn a_ray_through_the_centre_points_at_the_focus() {
        let (transform, projection) = camera();
        let (origin, direction) =
            ray_through(&transform, &projection, Vec2::new(0.5, 0.5)).expect("orthographic ray");

        assert!(
            origin.distance(Vec3::new(0.0, 0.0, 20.0)) < 1e-4,
            "{origin:?}"
        );
        // Looking at the origin from +Z means facing -Z.
        assert!(direction.distance(Vec3::NEG_Z) < 1e-5, "{direction:?}");
    }

    #[test]
    fn image_corners_map_to_the_right_corners_of_the_view() {
        let (transform, projection) = camera();
        // Top-left of the image is -x, +y in world space here.
        let (top_left, _) =
            ray_through(&transform, &projection, Vec2::ZERO).expect("orthographic ray");
        assert!(top_left.x < 0.0 && top_left.y > 0.0, "{top_left:?}");

        let (bottom_right, _) =
            ray_through(&transform, &projection, Vec2::ONE).expect("orthographic ray");
        assert!(
            bottom_right.x > 0.0 && bottom_right.y < 0.0,
            "{bottom_right:?}"
        );

        // The visible height is 10 units, so the corners span it exactly.
        assert!((top_left.y - bottom_right.y - 10.0).abs() < 1e-4);
    }

    #[test]
    fn a_ray_through_the_centre_finds_the_surface() {
        // The whole click path, minus egui: camera -> ray -> core space -> hit.
        let input = EllipsoidInput::default();
        let geometry = compute_geometry(&input);
        let param = SurfaceParam::new(&geometry);

        let (transform, projection) = camera();
        let (origin, direction) =
            ray_through(&transform, &projection, Vec2::new(0.5, 0.5)).expect("orthographic ray");

        let hit = ray_hit(&param, &geometry, from_bevy(origin), from_bevy(direction))
            .expect("a ray down the middle must hit the ellipsoid");

        assert!((0.0..=1.0).contains(&hit.u), "u out of range: {}", hit.u);
        assert!((0.0..=1.0).contains(&hit.v), "v out of range: {}", hit.v);

        // And the reported coordinate must resolve back to the same place.
        let back = surface_point(&param, &geometry, hit.u, hit.v);
        assert!(
            (back - hit.point).length() < 1e-6,
            "{back:?} vs {:?}",
            hit.point
        );
    }
}
