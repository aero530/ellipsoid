//! Two 3D views rendered to textures and displayed inside egui panels.
//!
//! Rendering off-screen rather than carving viewports out of the window means
//! egui owns the whole layout — panels can be resized and rearranged without
//! any camera-viewport arithmetic, and the 3D views cannot end up underneath a
//! panel the way the Phase 0 scaffold did.
//!
//! Each view gets its own render layer so the two cameras do not see each
//! other's mesh; lights are on both.

use bevy::camera::{RenderTarget, ScalingMode, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy_egui::{EguiGlobalSettings, EguiTextureHandle, EguiUserTextures, PrimaryEguiContext};
use bevy_panorbit_camera::{ActiveCameraData, PanOrbitCamera};
use ellipsoid_core::surface::SurfaceParam;
use ellipsoid_pattern::surface_domain;

use crate::mesh;
use crate::state::AppState;

/// Which of the two views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    /// The ellipsoid surface.
    Surface,
    /// The flattened pattern.
    Flat,
}

impl ViewKind {
    pub const ALL: [ViewKind; 2] = [ViewKind::Surface, ViewKind::Flat];

    pub fn title(self) -> &'static str {
        match self {
            ViewKind::Surface => "Ellipsoid",
            ViewKind::Flat => "Flattened",
        }
    }

    pub fn layer(self) -> usize {
        match self {
            ViewKind::Surface => 1,
            ViewKind::Flat => 2,
        }
    }
}

/// What a modified click in a viewport should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickAction {
    Add,
    Remove,
}

/// A click recorded by the UI, waiting to be turned into a ray.
#[derive(Debug, Clone, Copy)]
pub struct PendingPick {
    pub view: ViewKind,
    /// `(0,0)` top-left of the image, `(1,1)` bottom-right.
    pub normalized: Vec2,
    pub action: PickAction,
}

const INITIAL_SIZE: UVec2 = UVec2::new(512, 512);
/// Beyond this the off-screen targets cost more than the detail is worth.
const MAX_SIZE: u32 = 2048;
/// Ignore sub-threshold size changes so dragging a splitter does not reallocate
/// a texture every frame.
const RESIZE_THRESHOLD: u32 = 8;

pub struct ViewportTarget {
    pub kind: ViewKind,
    pub image: Handle<Image>,
    pub camera: Entity,
    pub mesh: Entity,
    /// Current texture size.
    pub size: UVec2,
    /// Size the UI would like, applied by [`apply_viewport_sizes`].
    pub desired: UVec2,
    /// Set by the UI each frame; drives which camera receives input.
    pub hovered: bool,
}

#[derive(Resource)]
pub struct Viewports {
    pub views: Vec<ViewportTarget>,
    /// A click awaiting a raycast; see [`crate::cutouts::apply_picks`].
    pub pending_pick: Option<PendingPick>,
    /// Last geometry generation the meshes were built from.
    last_generation: u64,
}

impl Viewports {
    pub fn get(&self, kind: ViewKind) -> &ViewportTarget {
        self.views
            .iter()
            .find(|v| v.kind == kind)
            .expect("both views are created at startup")
    }

    pub fn get_mut(&mut self, kind: ViewKind) -> &mut ViewportTarget {
        self.views
            .iter_mut()
            .find(|v| v.kind == kind)
            .expect("both views are created at startup")
    }
}

fn render_texture(size: UVec2) -> Image {
    let extent = Extent3d {
        width: size.x,
        height: size.y,
        depth_or_array_layers: 1,
    };
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("ellipsoid-viewport"),
            size: extent,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(extent);
    image
}

pub fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut user_textures: ResMut<EguiUserTextures>,
    mut egui_settings: ResMut<EguiGlobalSettings>,
) {
    // We supply the egui context ourselves, on a camera that renders nothing.
    egui_settings.auto_create_primary_context = false;

    let mut views = Vec::new();

    for (index, kind) in ViewKind::ALL.into_iter().enumerate() {
        let image = images.add(render_texture(INITIAL_SIZE));
        user_textures.add_image(EguiTextureHandle::Strong(image.clone()));

        let layer = RenderLayers::layer(kind.layer());

        // Back faces are always drawn — nothing may vanish because a triangle
        // happens to be wound away from the camera.
        //
        // Whether they are *lit* as if they faced the camera differs by view.
        // A flattened panel strip is legitimately viewed from either face, so
        // it stays double-sided, as the original's `THREE.DoubleSide` did. The
        // ellipsoid must not: flipping the normal shades the inside of the
        // shell exactly like the outside, which makes a hole cut through it
        // invisible. Lit by its true normal the interior falls into shadow and
        // the hole reads as one.
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb_u8(0x00, 0x87, 0xE6),
            perceptual_roughness: 0.6,
            double_sided: kind == ViewKind::Flat,
            cull_mode: None,
            ..default()
        });

        let mesh_entity = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(material),
                Transform::IDENTITY,
                layer.clone(),
            ))
            .id();

        let camera = commands
            .spawn((
                Camera3d::default(),
                Camera {
                    // Off-screen passes must run before the on-screen one.
                    order: -(index as isize) - 1,
                    clear_color: ClearColorConfig::Custom(Color::srgb(0.73, 0.73, 0.73)),
                    ..default()
                },
                RenderTarget::Image(image.clone().into()),
                // Orthographic like the original: no perspective distortion, so
                // panel shapes can be judged by eye.
                //
                // The extent stays at 1.0 because PanOrbitCamera drives zoom by
                // assigning `projection.scale = radius`, which multiplies it.
                // Leaving it at unity makes `PanOrbitCamera::radius` *be* the
                // visible vertical extent in world units, which is what
                // `sync_meshes` sets. near/far are left alone: panorbit derives
                // the camera distance from `(near + far) / 2`.
                Projection::from(OrthographicProjection {
                    scaling_mode: ScalingMode::FixedVertical {
                        viewport_height: 1.0,
                    },
                    ..OrthographicProjection::default_3d()
                }),
                PanOrbitCamera {
                    yaw: Some(0.7),
                    pitch: Some(0.4),
                    ..default()
                },
                // Per-view component in Bevy 0.19, not a resource.
                AmbientLight {
                    brightness: 600.0,
                    ..default()
                },
                layer.clone(),
            ))
            .id();

        views.push(ViewportTarget {
            kind,
            image,
            camera,
            mesh: mesh_entity,
            size: INITIAL_SIZE,
            desired: INITIAL_SIZE,
            hovered: false,
        });
    }

    // Three-point-ish lighting on both layers.
    //
    // The original used spot lights at fixed distances; directional lights give
    // the same character without depending on how far away the subject is,
    // which matters when the ellipsoid can be any size.
    let both = RenderLayers::from_layers(&[ViewKind::Surface.layer(), ViewKind::Flat.layer()]);
    for (direction, illuminance) in [
        (Vec3::new(0.0, 5.0, 2.0), 6_000.0),
        (Vec3::new(-2.0, -0.5, 2.0), 3_000.0),
        (Vec3::new(2.0, -0.5, -2.0), 3_000.0),
    ] {
        commands.spawn((
            DirectionalLight {
                illuminance,
                ..default()
            },
            Transform::from_translation(direction).looking_at(Vec3::ZERO, Vec3::Y),
            both.clone(),
        ));
    }

    // The egui context lives on its own camera which draws no 3D at all: egui
    // covers the window, and the 3D content arrives as textures.
    commands.spawn((
        PrimaryEguiContext,
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        RenderLayers::none(),
    ));

    commands.insert_resource(Viewports {
        views,
        pending_pick: None,
        last_generation: u64::MAX,
    });
}

/// Rebuild both meshes when the geometry changes, and refit the cameras.
pub fn sync_meshes(
    state: Res<AppState>,
    mut viewports: ResMut<Viewports>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_handles: Query<&mut Mesh3d>,
    mut cameras: Query<(&mut PanOrbitCamera, &mut Projection)>,
) {
    let Some(derived) = &state.derived else {
        return;
    };
    if viewports.last_generation == state.geometry_generation {
        return;
    }
    viewports.last_generation = state.geometry_generation;

    // Both views cut the same shapes out of the same domain, so the hole a
    // pattern shows and the hole the preview shows cannot drift apart.
    let param = SurfaceParam::new(&derived.geometry);
    let domain = surface_domain(&param, &derived.flat, &state.input.cutouts);

    for view in &viewports.views {
        let mesh = match view.kind {
            ViewKind::Surface => mesh::surface_mesh(&derived.geometry, &param, &domain),
            ViewKind::Flat => mesh::flat_mesh(&derived.flat, &param, &domain),
        };

        let (center, radius) = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .map(|p| {
                let pts: Vec<Vec3> = p.iter().map(|v| Vec3::from(*v)).collect();
                mesh::bounding_sphere(&pts)
            })
            .unwrap_or((Vec3::ZERO, 1.0));

        if let Ok(mut handle) = mesh_handles.get_mut(view.mesh) {
            handle.0 = meshes.add(mesh);
        }

        // Frame the subject the way the original did: an orthographic view just
        // larger than the bounding sphere, orbiting its centre.
        //
        // `radius` is half the bounding-box diagonal, so a visible extent of
        // 2.2x it fits the subject from any orbit angle with a little margin.
        if let Ok((mut orbit, _)) = cameras.get_mut(view.camera) {
            let extent = radius * 2.2;
            orbit.focus = center;
            orbit.target_focus = center;
            // Assigning both skips panorbit's smoothing, so a refit snaps
            // rather than drifting into place after every edit.
            orbit.radius = Some(extent);
            orbit.target_radius = extent;
        }
    }
}

/// Resize the off-screen textures to match the panels showing them.
pub fn apply_viewport_sizes(mut viewports: ResMut<Viewports>, mut images: ResMut<Assets<Image>>) {
    for view in &mut viewports.views {
        let desired = view.desired.clamp(UVec2::splat(16), UVec2::splat(MAX_SIZE));
        let delta = desired.as_ivec2() - view.size.as_ivec2();
        if delta.x.unsigned_abs() < RESIZE_THRESHOLD && delta.y.unsigned_abs() < RESIZE_THRESHOLD {
            continue;
        }
        if let Some(mut image) = images.get_mut(&view.image) {
            image.resize(Extent3d {
                width: desired.x,
                height: desired.y,
                depth_or_array_layers: 1,
            });
            view.size = desired;
        }
    }
}

/// Point the pan/orbit controller at whichever view the pointer is over.
///
/// `ActiveCameraData` is a single resource, so only one camera can be driven at
/// a time; with `manual: true` the plugin leaves it to us. Clearing `entity`
/// when nothing is hovered stops a drag over the settings panel from spinning
/// whichever view happened to be touched last.
pub fn route_camera_input(
    viewports: Res<Viewports>,
    windows: Query<&Window>,
    mut active: ResMut<ActiveCameraData>,
) {
    let window_size = windows
        .iter()
        .next()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::ONE);

    let hovered = viewports.views.iter().find(|v| v.hovered);
    active.set_if_neq(ActiveCameraData {
        entity: hovered.map(|v| v.camera),
        viewport_size: hovered.map(|v| v.size.as_vec2()),
        window_size: Some(window_size),
        manual: true,
    });
}
