//! Wavefront OBJ export — port of the `object3D` builders in
//! `app/utils/ellipsoid.js`.
//!
//! Faces are emitted as quads, one per surface patch, which keeps the meshes
//! readable in a text editor and matches what the original produced.
//!
//! # Number formatting
//!
//! Rust's `Display` for `f64` gives the shortest representation that round-trips,
//! same as JavaScript — but the two disagree on when to switch to exponent
//! notation. JavaScript writes `-2.88e-16` where Rust writes
//! `-0.000000000000000288`. Both parse identically and every OBJ reader accepts
//! either, so the values match even though the bytes do not. Tests compare
//! parsed numbers, never text.

// See the note in `geometry.rs`: index loops are kept so the vertex ordering
// here can be checked against the original's face-index arithmetic.
#![allow(clippy::needless_range_loop)]

use std::fmt::Write as _;

use crate::flatten::FlatGeometry;
use crate::geometry::Geometry;

/// The ellipsoid surface as an OBJ mesh.
pub fn geometry_to_obj(geometry: &Geometry) -> String {
    let phi_divisions = geometry.phi_divisions;
    let theta_divisions = geometry.theta_divisions;
    let points = &geometry.points;

    let mut out = String::with_capacity((phi_divisions + 1) * (theta_divisions + 1) * 48);

    for ip in 0..=phi_divisions {
        for it in 0..=theta_divisions {
            let p = points[ip][it];
            let _ = writeln!(out, "v {} {} {} ", p.x, p.y, p.z);
        }
    }

    // OBJ indices are 1-based. Each row of vertices is `theta_divisions + 1`
    // long, so stepping one phi line advances by that much — which is what the
    // `(ip-1)*divisions + (ip-1)` arithmetic works out to.
    for ip in 1..=phi_divisions {
        for it in 1..=theta_divisions {
            let a = it + (ip - 1) * theta_divisions + (ip - 1);
            let b = a + 1;
            let c = b + (theta_divisions + 1);
            let d = c - 1;
            let _ = writeln!(out, "f {a} {d} {c} {b} ");
        }
    }

    out
}

/// The flattened pattern as an OBJ mesh.
///
/// Unlike the surface mesh, each rung contributes its own pair of vertices —
/// panel strips are separate pieces once flattened, so they do not share edges.
pub fn flat_to_obj(flat: &FlatGeometry) -> String {
    let edges = &flat.edges_flat;
    let phi_divisions = edges.len();
    let theta_divisions = edges[0].len() - 1;

    let mut out = String::with_capacity(phi_divisions * (theta_divisions + 1) * 96);

    for ip in 0..phi_divisions {
        for it in 0..=theta_divisions {
            for side in 0..2 {
                let p = edges[ip][it][side];
                let _ = writeln!(out, "v {} {} {} ", p.x, p.y, p.z);
            }
        }
    }

    for ip in 0..phi_divisions {
        for it in 0..theta_divisions {
            let a = 1 + it * 2 + ip * theta_divisions * 2 + ip * 2;
            let b = a + 1;
            let c = a + 3;
            let d = a + 2;
            let _ = writeln!(out, "f {a} {b} {c} {d} ");
        }
    }

    out
}

/// Vertices and quad faces parsed out of an OBJ document.
///
/// Exposed for tests, which compare numbers rather than text.
pub fn parse_obj(text: &str) -> (Vec<[f64; 3]>, Vec<Vec<usize>>) {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => {
                let coords: Vec<f64> = parts.filter_map(|t| t.parse().ok()).collect();
                if coords.len() == 3 {
                    vertices.push([coords[0], coords[1], coords[2]]);
                }
            }
            Some("f") => {
                // Ignore any `v/vt/vn` decoration; these meshes are position-only.
                let idx: Vec<usize> = parts
                    .filter_map(|t| t.split('/').next().and_then(|n| n.parse().ok()))
                    .collect();
                if !idx.is_empty() {
                    faces.push(idx);
                }
            }
            _ => {}
        }
    }

    (vertices, faces)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::EllipsoidInput;
    use crate::{compute_flat_geometry, compute_geometry};

    #[test]
    fn surface_mesh_has_one_vertex_per_grid_point() {
        let g = compute_geometry(&EllipsoidInput::default());
        let (vertices, faces) = parse_obj(&geometry_to_obj(&g));
        assert_eq!(
            vertices.len(),
            (g.phi_divisions + 1) * (g.theta_divisions + 1)
        );
        assert_eq!(faces.len(), g.phi_divisions * g.theta_divisions);
    }

    #[test]
    fn flat_mesh_has_two_vertices_per_rung() {
        let input = EllipsoidInput::default();
        let g = compute_geometry(&input);
        let f = compute_flat_geometry(&g, &input);
        let (vertices, faces) = parse_obj(&flat_to_obj(&f));
        assert_eq!(
            vertices.len(),
            g.phi_divisions * (g.theta_divisions + 1) * 2
        );
        assert_eq!(faces.len(), g.phi_divisions * g.theta_divisions);
    }

    #[test]
    fn every_face_index_is_in_range() {
        let input = EllipsoidInput::default();
        let g = compute_geometry(&input);
        let f = compute_flat_geometry(&g, &input);

        for obj in [geometry_to_obj(&g), flat_to_obj(&f)] {
            let (vertices, faces) = parse_obj(&obj);
            for face in &faces {
                assert_eq!(face.len(), 4, "faces are quads");
                for &i in face {
                    assert!(
                        i >= 1 && i <= vertices.len(),
                        "index {i} outside 1..={}",
                        vertices.len()
                    );
                }
            }
        }
    }

    #[test]
    fn faces_reference_four_distinct_vertices() {
        // A degenerate face would mean the index arithmetic collapsed.
        let g = compute_geometry(&EllipsoidInput::default());
        let (_, faces) = parse_obj(&geometry_to_obj(&g));
        for face in faces {
            let mut sorted = face.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), 4, "degenerate face {face:?}");
        }
    }
}
