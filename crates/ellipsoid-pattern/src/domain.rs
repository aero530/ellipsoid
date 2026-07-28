//! The surface's `(u, v)` domain, triangulated with cutouts removed.
//!
//! Both 3D views draw the surface as a grid of quads over `(u, v)`, so a cutout
//! that has been subtracted from the flat pattern has to be subtracted from
//! those quads too, or the preview shows a solid shell where the pattern has a
//! hole.
//!
//! # Cell by cell, and triangle by triangle
//!
//! Subtracting once from the whole domain would work, but it would also
//! re-triangulate the entire surface to remove one 3 mm hole. A cutout is
//! local, so the subtraction is done per grid cell and only the cells it
//! actually reaches pay for it — everything else keeps the two triangles it
//! always had.
//!
//! Each cell is split along its diagonal *first*. The map from `(u, v)` to
//! either 3D view is affine on each of a quad's two triangles and generally
//! *not* affine across the pair, so a triangulation that ignored the diagonal
//! would place its vertices off the surface. Splitting first means every
//! triangle that comes out lies inside a single affine piece.

use ellipsoid_core::flatten::FlatGeometry;
use ellipsoid_core::surface::{Cutout, SurfaceParam};
use glam::DVec2;

use crate::cutouts::{make_ccw, rim_uv};

/// One triangle of the domain, tagged with the grid cell it came from.
///
/// The cell is not just provenance: the flat pattern keeps panel strips as
/// separate pieces, so placing a point that sits on a seam needs its strip
/// pinned rather than derived from `u` (see [`SurfaceParam::cell_in_strip`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomainTriangle {
    pub ip: usize,
    pub it: usize,
    /// Corners in surface coordinates.
    pub uv: [DVec2; 3],
}

/// Axis-aligned bounds, used only to reject cells a cutout cannot reach.
#[derive(Clone, Copy)]
struct Bounds {
    min: DVec2,
    max: DVec2,
}

impl Bounds {
    fn of(points: &[DVec2]) -> Self {
        let mut min = DVec2::splat(f64::INFINITY);
        let mut max = DVec2::splat(f64::NEG_INFINITY);
        for p in points {
            min = min.min(*p);
            max = max.max(*p);
        }
        Self { min, max }
    }

    fn overlaps(&self, other: &Bounds) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }
}

/// The surface grid with every cutout subtracted from it.
///
/// With no cutouts this is exactly the plain grid: two triangles per cell, in
/// the usual order.
pub fn surface_domain(
    param: &SurfaceParam,
    flat: &FlatGeometry,
    cutouts: &[Cutout],
) -> Vec<DomainTriangle> {
    let rims = wrapped_rims(param, flat, cutouts);

    let phi = param.phi_divisions;
    let theta = param.theta_divisions;
    let mut out = Vec::with_capacity(phi * theta * 2);

    for ip in 0..phi {
        let u0 = ip as f64 / phi as f64;
        let u1 = (ip + 1) as f64 / phi as f64;
        for it in 0..theta {
            let v0 = param.coord(ip, it, 0.0, 0.0).1;
            let v1 = param.coord(ip, it, 0.0, 1.0).1;

            // Corner order matches the quad everywhere else: v00 v10 v11 v01.
            let corners = [
                DVec2::new(u0, v0),
                DVec2::new(u1, v0),
                DVec2::new(u1, v1),
                DVec2::new(u0, v1),
            ];
            for indices in [[0, 1, 2], [0, 2, 3]] {
                let triangle = [
                    corners[indices[0]],
                    corners[indices[1]],
                    corners[indices[2]],
                ];
                let bounds = Bounds::of(&triangle);
                let reaching: Vec<&Vec<DVec2>> = rims
                    .iter()
                    .filter(|(_, b)| b.overlaps(&bounds))
                    .map(|(r, _)| r)
                    .collect();

                if reaching.is_empty() {
                    out.push(DomainTriangle {
                        ip,
                        it,
                        uv: triangle,
                    });
                    continue;
                }
                for piece in subtract(triangle, &reaching) {
                    out.push(DomainTriangle { ip, it, uv: piece });
                }
            }
        }
    }

    out
}

/// Every cutout's rim, plus the copies shifted one turn either way.
///
/// A shape may straddle `u = 0`, where the grid restarts. Offering the shifted
/// copies lets one subtraction reach cells at both ends of the range without
/// any special case for the wrap; the bounds test drops the copies that miss.
fn wrapped_rims(
    param: &SurfaceParam,
    flat: &FlatGeometry,
    cutouts: &[Cutout],
) -> Vec<(Vec<DVec2>, Bounds)> {
    let mut rims = Vec::new();
    for cutout in cutouts.iter().filter(|c| c.is_valid()) {
        let base = rim_uv(param, flat, cutout);
        if base.len() < 3 {
            continue;
        }
        for shift in [-1.0, 0.0, 1.0] {
            let moved: Vec<DVec2> = base.iter().map(|p| DVec2::new(p.x + shift, p.y)).collect();
            let bounds = Bounds::of(&moved);
            if bounds.max.x < 0.0 || bounds.min.x > 1.0 {
                continue;
            }
            rims.push((moved, bounds));
        }
    }
    rims
}

/// Cut `rims` out of one triangle and re-triangulate what survives.
///
/// A cutout that swallows the triangle whole legitimately returns nothing —
/// unlike [`crate::cutouts::subtract_from_outline`], where an empty result
/// would erase the pattern, here it is just a cell that is entirely hole.
fn subtract(triangle: [DVec2; 3], rims: &[&Vec<DVec2>]) -> Vec<[DVec2; 3]> {
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::core::overlay_rule::OverlayRule;
    use i_overlay::float::single::SingleFloatOverlay;

    let to_pairs = |pts: &[DVec2]| -> Vec<[f64; 2]> { pts.iter().map(|p| [p.x, p.y]).collect() };
    let subject = vec![to_pairs(&make_ccw(triangle.to_vec()))];
    let clip: Vec<Vec<[f64; 2]>> = rims
        .iter()
        .map(|r| to_pairs(&make_ccw((*r).clone())))
        .collect();

    let shapes = subject.overlay(&clip, OverlayRule::Difference, FillRule::NonZero);

    let mut out = Vec::new();
    for shape in shapes {
        // Contour 0 is the outer boundary; the rest are its holes, which is
        // exactly the layout earcut wants.
        let mut data: Vec<f64> = Vec::new();
        let mut holes: Vec<usize> = Vec::new();
        for (index, contour) in shape.iter().enumerate() {
            if contour.len() < 3 {
                continue;
            }
            if index > 0 {
                holes.push(data.len() / 2);
            }
            for p in contour {
                data.push(p[0]);
                data.push(p[1]);
            }
        }
        if data.len() < 6 {
            continue;
        }
        // A failure here means one cell's worth of surface goes missing, which
        // is a far better outcome than emitting triangles across the hole.
        let Ok(indices) = earcutr::earcut(&data, &holes, 2) else {
            continue;
        };
        let vertex = |i: usize| DVec2::new(data[i * 2], data[i * 2 + 1]);
        for corner in indices.chunks_exact(3) {
            out.push([vertex(corner[0]), vertex(corner[1]), vertex(corner[2])]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipsoid_core::{EllipsoidInput, compute_flat_geometry, compute_geometry};

    fn setup() -> (SurfaceParam, FlatGeometry) {
        let input = EllipsoidInput::default();
        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);
        (SurfaceParam::new(&geometry), flat)
    }

    /// Twice the signed area of a `(u, v)` triangle.
    fn area2(t: &[DVec2; 3]) -> f64 {
        ((t[1] - t[0]).perp_dot(t[2] - t[0])).abs()
    }

    fn total_area(domain: &[DomainTriangle]) -> f64 {
        domain.iter().map(|t| area2(&t.uv)).sum::<f64>() / 2.0
    }

    #[test]
    fn an_uncut_surface_is_the_plain_grid() {
        let (param, flat) = setup();
        let domain = surface_domain(&param, &flat, &[]);
        assert_eq!(
            domain.len(),
            param.phi_divisions * param.theta_divisions * 2
        );
        // And it covers the whole unit square exactly.
        assert!((total_area(&domain) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_hole_removes_its_own_area_and_no_more() {
        let (param, flat) = setup();
        // Large enough that the removed area is well above the tolerance.
        let cutout = Cutout::hole(0.5 / 8.0, 0.5, 0.5);
        let rim = rim_uv(&param, &flat, &cutout);

        let mut expected = 0.0;
        for i in 0..rim.len() {
            let (a, b) = (rim[i], rim[(i + 1) % rim.len()]);
            expected += a.perp_dot(b);
        }
        let expected = expected.abs() / 2.0;

        let removed = 1.0 - total_area(&surface_domain(&param, &flat, &[cutout]));
        assert!(
            (removed - expected).abs() < expected * 1e-6,
            "removed {removed}, rim encloses {expected}"
        );
    }

    #[test]
    fn a_polygon_removes_its_own_area() {
        let (param, flat) = setup();
        let cutout = Cutout::polygon(vec![
            [0.20, 0.30],
            [0.40, 0.30],
            [0.40, 0.50],
            [0.30, 0.60],
            [0.20, 0.50],
        ]);
        // Shoelace on the vertices themselves: a polygon needs no rim fitting.
        let points = [
            DVec2::new(0.20, 0.30),
            DVec2::new(0.40, 0.30),
            DVec2::new(0.40, 0.50),
            DVec2::new(0.30, 0.60),
            DVec2::new(0.20, 0.50),
        ];
        let mut expected = 0.0;
        for i in 0..points.len() {
            expected += points[i].perp_dot(points[(i + 1) % points.len()]);
        }
        let expected = expected.abs() / 2.0;

        let removed = 1.0 - total_area(&surface_domain(&param, &flat, &[cutout]));
        assert!(
            (removed - expected).abs() < expected * 1e-6,
            "removed {removed}, polygon encloses {expected}"
        );
    }

    #[test]
    fn a_hole_on_the_wrap_is_cut_from_both_ends() {
        // u = 0 is a seam like any other for the pattern, but for the grid it is
        // where the range restarts: half the hole is near u = 1.
        let (param, flat) = setup();
        let domain = surface_domain(&param, &flat, &[Cutout::hole(0.0, 0.5, 0.5)]);

        let strip_area = |strip: usize| {
            domain
                .iter()
                .filter(|t| t.ip == strip)
                .map(|t| area2(&t.uv))
                .sum::<f64>()
                / 2.0
        };
        let plain = 1.0 / param.phi_divisions as f64;
        assert!(
            strip_area(0) < plain * 0.999,
            "nothing was cut from the first strip"
        );
        assert!(
            strip_area(param.phi_divisions - 1) < plain * 0.999,
            "nothing was cut from the last strip"
        );
    }

    #[test]
    fn only_the_cells_a_hole_reaches_are_retriangulated() {
        // The point of working cell by cell. A 3 mm hole must not disturb the
        // rest of the grid.
        let (param, flat) = setup();
        let diameter = Cutout::default_diameter(ellipsoid_core::Unit::Inch);
        let domain = surface_domain(&param, &flat, &[Cutout::hole(0.5 / 8.0, 0.5, diameter)]);

        let plain = param.phi_divisions * param.theta_divisions * 2;
        let mut cells = std::collections::HashSet::new();
        for t in &domain {
            cells.insert((t.ip, t.it));
        }
        assert_eq!(cells.len(), param.phi_divisions * param.theta_divisions);

        // A hole this size fits inside one triangle, so the whole cost is
        // fanning that triangle around a rim: on the order of `RIM_SEGMENTS`
        // extra triangles, not a re-triangulation of the grid.
        let budget = plain + 2 * crate::cutouts::RIM_SEGMENTS;
        assert!(
            domain.len() < budget,
            "a small hole re-triangulated too much: {} vs {plain} plain",
            domain.len()
        );
    }

    #[test]
    fn every_triangle_stays_inside_its_own_cell() {
        // Vertices leaking into a neighbouring cell would be placed with the
        // wrong strip pinned on the flat mesh, which is the seam bug from
        // Appendix I in a different guise.
        let (param, flat) = setup();
        let domain = surface_domain(
            &param,
            &flat,
            &[
                Cutout::hole(2.0 / 8.0, 0.5, 0.3),
                Cutout::polygon(vec![[0.6, 0.2], [0.8, 0.25], [0.7, 0.45]]),
            ],
        );

        for t in &domain {
            let u0 = t.ip as f64 / param.phi_divisions as f64;
            let u1 = (t.ip + 1) as f64 / param.phi_divisions as f64;
            let v0 = param.coord(t.ip, t.it, 0.0, 0.0).1;
            let v1 = param.coord(t.ip, t.it, 0.0, 1.0).1;
            for p in t.uv {
                assert!(
                    p.x >= u0 - 1e-9 && p.x <= u1 + 1e-9,
                    "u {} outside [{u0}, {u1}]",
                    p.x
                );
                assert!(
                    p.y >= v0 - 1e-9 && p.y <= v1 + 1e-9,
                    "v {} outside [{v0}, {v1}]",
                    p.y
                );
            }
        }
    }
}
