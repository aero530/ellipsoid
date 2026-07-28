//! Bevy meshes built straight from the core geometry.
//!
//! The original went `points → OBJ text → OBJLoader → mesh`, which existed only
//! because three.js had a loader handy. Building the mesh directly skips a
//! parse and a string allocation per edit.
//!
//! # These meshes are built from the cut domain, not the raw grid
//!
//! Both views take a [`DomainTriangle`] list — the `(u, v)` grid with the
//! cutouts already subtracted (`RUST_CONVERSION_PLAN.md` Appendix K) — so a
//! shape drawn on the pattern is genuinely missing from the preview rather than
//! marked on a solid shell.
//!
//! §7.3 originally planned to map a 3D pick back through the *mesh's* triangle
//! index, which would have made the ordering here a contract. It never came to
//! that: [`crate::cutouts`] raycasts the core geometry directly, so nothing
//! outside this module depends on how the triangles are laid out — which is
//! what leaves them free to be re-triangulated around a hole.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use ellipsoid_core::surface::{Cell, SurfaceParam, flat_point_in_strip, surface_point};
use ellipsoid_core::{DVec3, FlatGeometry, Geometry};
use ellipsoid_pattern::DomainTriangle;

/// Core is Z-up (`c` is the height axis); Bevy is Y-up.
///
/// A -90° rotation about X, so it is a proper rotation rather than a mirror and
/// face winding is preserved. Exported OBJ stays Z-up like the original — only
/// the on-screen orientation changes.
pub fn to_bevy(p: DVec3) -> Vec3 {
    Vec3::new(p.x as f32, p.z as f32, -p.y as f32)
}

/// The inverse of [`to_bevy`], for taking picked rays back into core space.
pub fn from_bevy(p: Vec3) -> DVec3 {
    DVec3::new(p.x as f64, -p.z as f64, p.y as f64)
}

/// Two triangles per quad: `(v00, v10, v11)` then `(v00, v11, v01)`, matching
/// the split [`ellipsoid_core::surface`] uses to interpolate across a cell.
fn quad_indices(
    positions_per_row: usize,
    phi_divisions: usize,
    theta_divisions: usize,
) -> Vec<u32> {
    let mut indices = Vec::with_capacity(phi_divisions * theta_divisions * 6);
    for ip in 0..phi_divisions {
        for it in 0..theta_divisions {
            let v00 = (ip * positions_per_row + it) as u32;
            let v01 = v00 + 1;
            let v10 = ((ip + 1) * positions_per_row + it) as u32;
            let v11 = v10 + 1;
            indices.extend_from_slice(&[v00, v10, v11]);
            indices.extend_from_slice(&[v00, v11, v01]);
        }
    }
    indices
}

fn build(positions: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices))
    .with_computed_smooth_normals()
}

/// The plain quad grid, before any cutout is taken out of it.
fn grid(
    positions: Vec<Vec3>,
    per_row: usize,
    phi_divisions: usize,
    theta_divisions: usize,
) -> Mesh {
    let indices = quad_indices(per_row, phi_divisions, theta_divisions);
    build(positions, indices)
}

/// Read back the smooth normals [`build`] computed, one per grid vertex.
///
/// Cut cells cannot share vertices the way the grid does — a hole boundary puts
/// a vertex wherever it crosses — so `with_computed_smooth_normals` on the cut
/// mesh would shade it faceted. Interpolating *these* instead reproduces the
/// uncut shading exactly: the same normals at the grid points, blended by the
/// same piecewise-affine map that places the positions.
fn grid_normals(mesh: &Mesh) -> Vec<Vec3> {
    mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|a| a.as_float3())
        .map(|n| n.iter().map(|v| Vec3::from(*v)).collect())
        .unwrap_or_default()
}

/// Blend four corner values the way [`ellipsoid_core::surface`] blends corners.
///
/// Must match the triangle split there, or a normal would be interpolated
/// across a diagonal the position never crosses.
fn blend(corners: [Vec3; 4], cell: &Cell) -> Vec3 {
    let w = cell.weights.map(|x| x as f32);
    match cell.triangle {
        0 => corners[0] * w[0] + corners[1] * w[1] + corners[2] * w[2],
        _ => corners[0] * w[0] + corners[2] * w[1] + corners[3] * w[2],
    }
}

/// The ellipsoid surface, with `domain`'s cutouts taken out of it.
pub fn surface_mesh(geometry: &Geometry, param: &SurfaceParam, domain: &[DomainTriangle]) -> Mesh {
    let per_row = geometry.theta_divisions + 1;
    let grid_positions: Vec<Vec3> = geometry
        .points
        .iter()
        .flat_map(|row| row.iter().map(|p| to_bevy(*p)))
        .collect();
    let normals = grid_normals(&grid(
        grid_positions,
        per_row,
        geometry.phi_divisions,
        geometry.theta_divisions,
    ));
    let normal_at =
        |ip: usize, it: usize| normals.get(ip * per_row + it).copied().unwrap_or(Vec3::Y);

    let mut positions = Vec::with_capacity(domain.len() * 3);
    let mut vertex_normals = Vec::with_capacity(domain.len() * 3);
    for triangle in domain {
        for uv in triangle.uv {
            // The surface grid shares its columns, so resolving the strip from
            // `u` is safe here — unlike the flat mesh below.
            let cell = param.cell(uv.x, uv.y);
            positions.push(to_bevy(surface_point(param, geometry, uv.x, uv.y)));
            vertex_normals.push(
                blend(
                    [
                        normal_at(cell.ip, cell.it),
                        normal_at(cell.ip + 1, cell.it),
                        normal_at(cell.ip + 1, cell.it + 1),
                        normal_at(cell.ip, cell.it + 1),
                    ],
                    &cell,
                )
                .normalize_or_zero(),
            );
        }
    }

    let indices = Indices::U32((0..positions.len() as u32).collect());
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vertex_normals)
    .with_inserted_indices(indices)
}

/// The flattened pattern, with `domain`'s cutouts taken out of it.
///
/// Each rung contributes both of its points, so panel strips stay separate
/// pieces rather than sharing edges — the same layout the flat OBJ uses, and
/// the reason every point here is placed with its own strip pinned.
pub fn flat_mesh(flat: &FlatGeometry, param: &SurfaceParam, domain: &[DomainTriangle]) -> Mesh {
    let mut positions = Vec::with_capacity(domain.len() * 3);
    for triangle in domain {
        for uv in triangle.uv {
            positions.push(to_bevy(flat_point_in_strip(
                param,
                flat,
                triangle.ip as i64,
                uv.x,
                uv.y,
            )));
        }
    }
    let indices = (0..positions.len() as u32).collect();
    // The pattern lies in a plane, so per-face normals and smooth normals come
    // out the same; no interpolation needed.
    build(positions, indices)
}

/// Centre and radius of a bounding sphere around the mesh's bounding box.
///
/// Matches how the original sized its camera: `BoxHelper` around the mesh, then
/// that box's bounding sphere.
pub fn bounding_sphere(positions: &[Vec3]) -> (Vec3, f32) {
    if positions.is_empty() {
        return (Vec3::ZERO, 1.0);
    }
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for p in positions {
        min = min.min(*p);
        max = max.max(*p);
    }
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(1e-3);
    (center, radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipsoid_core::surface::Cutout;
    use ellipsoid_core::{EllipsoidInput, compute_flat_geometry, compute_geometry};
    use ellipsoid_pattern::surface_domain;

    struct Setup {
        geometry: Geometry,
        flat: FlatGeometry,
        param: SurfaceParam,
    }

    fn setup() -> Setup {
        let input = EllipsoidInput::default();
        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);
        let param = SurfaceParam::new(&geometry);
        Setup {
            geometry,
            flat,
            param,
        }
    }

    #[test]
    fn both_meshes_have_a_vertex_per_domain_corner() {
        let s = setup();
        let domain = surface_domain(&s.param, &s.flat, &[]);
        assert_eq!(
            domain.len(),
            s.geometry.phi_divisions * s.geometry.theta_divisions * 2
        );

        for mesh in [
            surface_mesh(&s.geometry, &s.param, &domain),
            flat_mesh(&s.flat, &s.param, &domain),
        ] {
            assert_eq!(mesh.count_vertices(), domain.len() * 3);
        }
    }

    #[test]
    fn a_hole_takes_its_own_area_out_of_both_meshes() {
        // Not just "smaller": the 3D views must lose exactly the hole that gets
        // drawn on the page, or the preview lies about what will be cut.
        let s = setup();
        let diameter = 0.4;
        let plain = surface_domain(&s.param, &s.flat, &[]);
        let cut = surface_domain(&s.param, &s.flat, &[Cutout::hole(0.5 / 8.0, 0.5, diameter)]);

        let expected = std::f32::consts::PI * (diameter as f32 / 2.0).powi(2);
        for (name, before, after) in [
            (
                "surface",
                surface_mesh(&s.geometry, &s.param, &plain),
                surface_mesh(&s.geometry, &s.param, &cut),
            ),
            (
                "flat",
                flat_mesh(&s.flat, &s.param, &plain),
                flat_mesh(&s.flat, &s.param, &cut),
            ),
        ] {
            // The hole is sized on the page, so the flat mesh loses it exactly;
            // the surface is curved across it, so allow a little more there.
            let removed = mesh_area(&before) - mesh_area(&after);
            assert!(
                (removed - expected).abs() < expected * 0.1,
                "{name} lost {removed}, expected {expected}"
            );
        }
    }

    /// Total triangle area of an unindexed mesh.
    fn mesh_area(mesh: &Mesh) -> f32 {
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .expect("positions");
        positions
            .chunks_exact(3)
            .map(|t| {
                let (a, b, c) = (Vec3::from(t[0]), Vec3::from(t[1]), Vec3::from(t[2]));
                (b - a).cross(c - a).length() * 0.5
            })
            .sum()
    }

    #[test]
    fn cutting_a_hole_does_not_change_the_shading_elsewhere() {
        // The reason the normals are interpolated rather than recomputed: a
        // hole must not make the rest of the surface look faceted.
        let s = setup();
        let plain = surface_domain(&s.param, &s.flat, &[]);
        let cut = surface_domain(&s.param, &s.flat, &[Cutout::hole(0.5 / 8.0, 0.5, 0.2)]);

        let normal_at = |mesh: &Mesh, position: Vec3| -> Option<Vec3> {
            let p = mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3())?;
            let n = mesh
                .attribute(Mesh::ATTRIBUTE_NORMAL)
                .and_then(|a| a.as_float3())?;
            p.iter()
                .position(|v| Vec3::from(*v).distance(position) < 1e-5)
                .map(|i| Vec3::from(n[i]))
        };

        // A grid corner on the far side of the ellipsoid from the hole.
        let (u, v) = s.param.coord(4, 8, 0.0, 0.0);
        let far = to_bevy(surface_point(&s.param, &s.geometry, u, v));
        let before = normal_at(&surface_mesh(&s.geometry, &s.param, &plain), far)
            .expect("the point is a grid corner");
        let after =
            normal_at(&surface_mesh(&s.geometry, &s.param, &cut), far).expect("still a corner");
        assert!(
            before.distance(after) < 1e-5,
            "shading moved: {before:?} vs {after:?}"
        );
    }

    #[test]
    fn axis_conversion_is_a_proper_rotation() {
        // Right-handedness must survive, or winding flips.
        let x = to_bevy(DVec3::X);
        let y = to_bevy(DVec3::Y);
        let z = to_bevy(DVec3::Z);
        assert!((x.cross(y) - z).length() < 1e-6, "{x:?} x {y:?} != {z:?}");
    }

    #[test]
    fn bounding_sphere_covers_the_points() {
        let pts = vec![
            Vec3::new(-1.0, -2.0, -3.0),
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::ZERO,
        ];
        let (center, radius) = bounding_sphere(&pts);
        assert_eq!(center, Vec3::ZERO);
        for p in &pts {
            assert!(p.distance(center) <= radius + 1e-6);
        }
    }
}
