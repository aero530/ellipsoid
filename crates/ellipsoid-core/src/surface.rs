//! Surface coordinates, and the mapping between the 3D surface and the flat
//! pattern.
//!
//! This is the machinery behind cutouts (`RUST_CONVERSION_PLAN.md` §7.3). The
//! key observation is that the surface grid and the flattened grid share a
//! topology — same quads, same two triangles per quad, same order — so a point
//! identified by *which triangle* and *where in it* transfers between them
//! exactly. No homography, no inverse bilinear, no degenerate-quad handling:
//! the map is piecewise-affine, defined by the triangulation itself.
//!
//! # Storage is resolution-independent
//!
//! A cutout stores normalised surface coordinates, not a triangle index:
//!
//! - `u` runs `0..1` around phi (and wraps),
//! - `v` runs `0..1` from the bottom edge to the top, **by arc length** along
//!   the surface profile.
//!
//! Storing the triangle would break the moment `divisions` changed — the hole
//! would jump. Index-space `v` would too, less obviously: `compute_geometry`
//! inserts up to three extra theta rows (the equator, the `h_middle` split, the
//! `h_bottom` ring) whose *count* is fixed, so their share of the index range
//! shrinks as `theta_divisions` grows, sliding everything above them. Arc
//! length has no such dependency.
//!
//! Changing `a`/`b`/`c` keeps holes at the same place *on the surface* rather
//! than at a fixed point in space, which is the decision recorded in §12.6.

use serde::{Deserialize, Serialize};

use crate::flatten::FlatGeometry;
use crate::geometry::Geometry;
use crate::{DVec3, Unit};

/// Something to cut out of the finished pattern.
///
/// Serialised untagged, so a settings file written when the only shape was a
/// round hole still loads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cutout {
    /// A round hole, sized in the document's [`Unit`].
    Hole {
        /// Position around the circumference, `0..1`, wrapping.
        u: f64,
        /// Position from bottom to top by arc length, `0..1`.
        v: f64,
        diameter: f64,
    },
    /// A free-form outline. Vertices are surface coordinates, so the shape
    /// follows the surface when the ellipsoid is reshaped, exactly as a hole's
    /// centre does.
    Polygon { points: Vec<[f64; 2]> },
}

impl Cutout {
    /// The default hole size, 3 mm, expressed in `unit`.
    ///
    /// Lengths elsewhere are plain numbers interpreted in the active unit —
    /// switching units reinterprets rather than converts, which is the legacy
    /// behaviour — so the 3 mm default is converted once, when the cutout is
    /// placed (§12.7).
    pub fn default_diameter(unit: Unit) -> f64 {
        3.0 * Unit::Mm.px_per_unit() / unit.px_per_unit()
    }

    /// A round hole, with its position normalised.
    pub fn hole(u: f64, v: f64, diameter: f64) -> Self {
        Cutout::Hole {
            u: u.rem_euclid(1.0),
            v: v.clamp(0.0, 1.0),
            diameter,
        }
    }

    pub fn polygon(points: Vec<[f64; 2]>) -> Self {
        Cutout::Polygon { points }
    }

    /// A single representative point: the centre, or the vertex average.
    ///
    /// Used for hit-testing and for reporting where a cutout is.
    pub fn anchor(&self) -> (f64, f64) {
        match self {
            Cutout::Hole { u, v, .. } => (*u, *v),
            Cutout::Polygon { points } => {
                if points.is_empty() {
                    return (0.0, 0.0);
                }
                let n = points.len() as f64;
                let (su, sv) = points
                    .iter()
                    .fold((0.0, 0.0), |(su, sv), p| (su + p[0], sv + p[1]));
                // Wrapped only here, and only after averaging: averaging
                // already-wrapped values is what puts the anchor of a shape
                // straddling `u = 0` on the opposite side of the pattern.
                ((su / n).rem_euclid(1.0), sv / n)
            }
        }
    }

    /// Shift the whole shape across the surface.
    pub fn translate(&mut self, du: f64, dv: f64) {
        match self {
            Cutout::Hole { u, v, .. } => {
                *u = (*u + du).rem_euclid(1.0);
                *v = (*v + dv).clamp(0.0, 1.0);
            }
            Cutout::Polygon { points } => {
                // Move as a unit: clamp the shift so no vertex leaves the
                // pattern, rather than clamping each vertex and deforming it.
                let (lo, hi) = points.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                    (lo.min(p[1]), hi.max(p[1]))
                });
                let dv = dv.clamp(-lo, 1.0 - hi);
                for p in points.iter_mut() {
                    // No wrapping per vertex — see `renormalise`.
                    p[0] += du;
                    p[1] += dv;
                }
                renormalise(points);
            }
        }
    }

    /// Every value that must be finite for the shape to be usable.
    pub fn is_valid(&self) -> bool {
        match self {
            Cutout::Hole { u, v, diameter } => {
                u.is_finite() && v.is_finite() && diameter.is_finite() && *diameter > 0.0
            }
            Cutout::Polygon { points } => {
                points.len() >= 3 && points.iter().all(|p| p[0].is_finite() && p[1].is_finite())
            }
        }
    }

    /// Bring a polygon's vertices back within reach of `0..1` without tearing
    /// it, after something has moved them.
    ///
    /// Call this rather than wrapping vertices individually. `u` wraps, so a
    /// shape can legitimately straddle `u = 0` — and then some of its vertices
    /// belong past 1 or below 0. Wrapping each one on its own splits the shape
    /// across the full width of the pattern instead: an 0.08-wide square dragged
    /// over the seam became 0.92 wide, and its anchor jumped to the far side.
    ///
    /// Everything downstream is built for the coherent form — `ellipsoid_pattern`
    /// indexes strips with a signed integer, and the 3D domain offers each rim
    /// shifted a turn either way — so the whole shape is moved by whole turns
    /// only, and its width never changes.
    pub fn renormalise_wrap(&mut self) {
        if let Cutout::Polygon { points } = self {
            renormalise(points);
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Cutout::Hole { .. } => "hole",
            Cutout::Polygon { .. } => "shape",
        }
    }
}

/// Shift a polygon by whole turns so its mean `u` lands in `0..1`.
///
/// Whole turns only, so the shape's width and its vertices' relationship to one
/// another are untouched. See [`Cutout::renormalise_wrap`].
fn renormalise(points: &mut [[f64; 2]]) {
    if points.is_empty() {
        return;
    }
    let mean = points.iter().map(|p| p[0]).sum::<f64>() / points.len() as f64;
    let turns = mean.div_euclid(1.0);
    if turns != 0.0 {
        for p in points {
            p[0] -= turns;
        }
    }
}

/// A located point: which quad, which of its two triangles, and the barycentric
/// weights within that triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub ip: usize,
    pub it: usize,
    /// 0 for `(v00, v10, v11)`, 1 for `(v00, v11, v01)`.
    pub triangle: usize,
    pub weights: [f64; 3],
}

impl Cell {
    /// Position within the quad: `s` along phi, `t` along theta, both `0..1`.
    pub fn in_quad(&self) -> (f64, f64) {
        let w = self.weights;
        match self.triangle {
            0 => (w[1] + w[2], w[2]),
            _ => (w[1], w[1] + w[2]),
        }
    }
}

/// Where a ray met the surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    pub u: f64,
    pub v: f64,
    /// The intersection point itself, in surface space.
    pub point: DVec3,
    /// Distance along the ray.
    pub distance: f64,
}

/// The `(u, v)` parametrisation of a particular tessellation.
///
/// Holds the arc-length profile that makes `v` independent of division counts.
/// Cheap to build — one pass over the grid — so it is rebuilt with the geometry
/// rather than cached.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceParam {
    pub phi_divisions: usize,
    pub theta_divisions: usize,
    /// Cumulative arc length at each theta row; length `theta_divisions + 1`.
    cumulative: Vec<f64>,
}

impl SurfaceParam {
    /// Measure the profile of `geometry`.
    ///
    /// Segment lengths are averaged across all phi columns rather than sampled
    /// from one, so a column that happens to be degenerate cannot skew the
    /// parametrisation.
    pub fn new(geometry: &Geometry) -> Self {
        let phi = geometry.phi_divisions;
        let theta = geometry.theta_divisions;

        let mut cumulative = Vec::with_capacity(theta + 1);
        cumulative.push(0.0);
        let mut total = 0.0;
        for it in 0..theta {
            let mut sum = 0.0;
            for column in geometry.points.iter().take(phi) {
                sum += (column[it + 1] - column[it]).length();
            }
            total += sum / phi.max(1) as f64;
            cumulative.push(total);
        }

        Self {
            phi_divisions: phi,
            theta_divisions: theta,
            cumulative,
        }
    }

    fn total_length(&self) -> f64 {
        *self.cumulative.last().unwrap_or(&0.0)
    }

    /// `v` to a fractional theta-row index.
    fn theta_index(&self, v: f64) -> f64 {
        let total = self.total_length();
        if total <= 0.0 {
            // Degenerate profile: fall back to index space.
            return v.clamp(0.0, 1.0) * self.theta_divisions as f64;
        }
        let target = v.clamp(0.0, 1.0) * total;
        // `cumulative` is sorted, so a binary search finds the segment.
        let hi = self
            .cumulative
            .partition_point(|&c| c <= target)
            .clamp(1, self.theta_divisions);
        let lo = hi - 1;
        let span = self.cumulative[hi] - self.cumulative[lo];
        let frac = if span > 0.0 {
            (target - self.cumulative[lo]) / span
        } else {
            0.0
        };
        lo as f64 + frac
    }

    /// A fractional theta-row index back to `v`.
    fn v_of_index(&self, index: f64) -> f64 {
        let total = self.total_length();
        if total <= 0.0 {
            return (index / self.theta_divisions.max(1) as f64).clamp(0.0, 1.0);
        }
        let clamped = index.clamp(0.0, self.theta_divisions as f64);
        let lo = (clamped.floor() as usize).min(self.theta_divisions.saturating_sub(1));
        let frac = clamped - lo as f64;
        let length = self.cumulative[lo] + (self.cumulative[lo + 1] - self.cumulative[lo]) * frac;
        (length / total).clamp(0.0, 1.0)
    }

    /// Resolve `(u, v)` to a quad, a triangle, and barycentric weights.
    ///
    /// The quad's corners are, in order, `v00 v10 v11 v01`, where the first
    /// digit steps along phi and the second along theta — matching the mesh
    /// built in the app and the OBJ face order.
    pub fn cell(&self, u: f64, v: f64) -> Cell {
        let fu = u.rem_euclid(1.0) * self.phi_divisions as f64;
        let fv = self.theta_index(v);

        // `min` guards the top edge, where floor lands one cell past the end.
        let ip = (fu.floor() as usize).min(self.phi_divisions.saturating_sub(1));
        let it = (fv.floor() as usize).min(self.theta_divisions.saturating_sub(1));
        let s = (fu - ip as f64).clamp(0.0, 1.0);
        let t = (fv - it as f64).clamp(0.0, 1.0);

        // Triangle 0 spans (0,0), (1,0), (1,1): s = b1 + b2, t = b2, so s >= t.
        // Triangle 1 spans (0,0), (1,1), (0,1): s = b1, t = b1 + b2, so t >= s.
        if s >= t {
            Cell {
                ip,
                it,
                triangle: 0,
                weights: [1.0 - s, s - t, t],
            }
        } else {
            Cell {
                ip,
                it,
                triangle: 1,
                weights: [1.0 - t, s, t - s],
            }
        }
    }

    /// Total arc length of the surface profile, bottom to top.
    pub fn profile_length(&self) -> f64 {
        self.total_length()
    }

    /// How much of the `v` range theta row `it` occupies.
    pub fn row_v_span(&self, it: usize) -> f64 {
        let total = self.total_length();
        if total <= 0.0 || it + 1 >= self.cumulative.len() {
            return 1.0 / self.theta_divisions.max(1) as f64;
        }
        (self.cumulative[it + 1] - self.cumulative[it]) / total
    }

    /// Like [`Self::cell`], but with the phi strip pinned instead of derived
    /// from `u`.
    ///
    /// Two reasons this is needed.
    ///
    /// A point exactly *on* a strip boundary is ambiguous — `cell` always
    /// resolves it to the strip on the right. For a shape being clipped to the
    /// strip on the left, that silently places its cut edge on the wrong panel,
    /// which at the phi wrap means the opposite end of the page.
    ///
    /// And `s` is deliberately not clamped, so passing a `u` outside the strip
    /// gives that strip's *affine extension*. Unlike the true map, that is
    /// continuous across a boundary, which is what makes it usable as a frame
    /// for fitting a shape that spans one.
    pub fn cell_in_strip(&self, strip: i64, u: f64, v: f64) -> Cell {
        let s = u * self.phi_divisions as f64 - strip as f64;
        let fv = self.theta_index(v);
        let it = (fv.floor() as usize).min(self.theta_divisions.saturating_sub(1));
        let t = (fv - it as f64).clamp(0.0, 1.0);
        let ip = strip.rem_euclid(self.phi_divisions as i64) as usize;

        if s >= t {
            Cell {
                ip,
                it,
                triangle: 0,
                weights: [1.0 - s, s - t, t],
            }
        } else {
            Cell {
                ip,
                it,
                triangle: 1,
                weights: [1.0 - t, s, t - s],
            }
        }
    }

    /// The inverse of [`Self::cell`]: grid position to `(u, v)`.
    pub fn coord(&self, ip: usize, it: usize, s: f64, t: f64) -> (f64, f64) {
        (
            (ip as f64 + s) / self.phi_divisions as f64,
            self.v_of_index(it as f64 + t),
        )
    }
}

/// Combine a cell's weights with a quad's four corners.
fn blend(corners: [DVec3; 4], cell: &Cell) -> DVec3 {
    let w = cell.weights;
    match cell.triangle {
        0 => corners[0] * w[0] + corners[1] * w[1] + corners[2] * w[2],
        _ => corners[0] * w[0] + corners[2] * w[1] + corners[3] * w[2],
    }
}

fn surface_corners(geometry: &Geometry, ip: usize, it: usize) -> [DVec3; 4] {
    [
        geometry.points[ip][it],
        geometry.points[ip + 1][it],
        geometry.points[ip + 1][it + 1],
        geometry.points[ip][it + 1],
    ]
}

fn flat_corners(flat: &FlatGeometry, ip: usize, it: usize) -> [DVec3; 4] {
    // edges_flat[ip][it] = [phi line ip, phi line ip+1] at rung it, so the
    // corners line up with the surface quad above.
    [
        flat.edges_flat[ip][it][0],
        flat.edges_flat[ip][it][1],
        flat.edges_flat[ip][it + 1][1],
        flat.edges_flat[ip][it + 1][0],
    ]
}

/// The 3D point at a surface coordinate.
pub fn surface_point(param: &SurfaceParam, geometry: &Geometry, u: f64, v: f64) -> DVec3 {
    let cell = param.cell(u, v);
    blend(surface_corners(geometry, cell.ip, cell.it), &cell)
}

/// The flat-pattern point corresponding to a surface coordinate.
///
/// The same barycentric position, evaluated on the matching flat triangle.
pub fn flat_point(param: &SurfaceParam, flat: &FlatGeometry, u: f64, v: f64) -> DVec3 {
    let cell = param.cell(u, v);
    blend(flat_corners(flat, cell.ip, cell.it), &cell)
}

/// [`flat_point`] with the phi strip pinned. See [`SurfaceParam::cell_in_strip`].
pub fn flat_point_in_strip(
    param: &SurfaceParam,
    flat: &FlatGeometry,
    strip: i64,
    u: f64,
    v: f64,
) -> DVec3 {
    let cell = param.cell_in_strip(strip, u, v);
    blend(flat_corners(flat, cell.ip, cell.it), &cell)
}

/// [`flat_jacobian`] with the phi strip pinned.
pub fn flat_jacobian_in_strip(
    param: &SurfaceParam,
    flat: &FlatGeometry,
    strip: i64,
    u: f64,
    v: f64,
) -> FlatJacobian {
    jacobian_of(param, flat, param.cell_in_strip(strip, u, v))
}

/// How a step in `(u, v)` moves a point **on the drawn page**.
///
/// A full 2×2, with columns `∂/∂u` and `∂/∂v`:
///
/// ```text
/// [ dx ] = [ m00  m01 ] [ du ]
/// [ dy ]   [ m10  m11 ] [ dv ]
/// ```
///
/// Two things make the off-diagonal terms load-bearing. Gores taper, so a
/// strip's sides slope relative to its rungs and `u`/`v` are not perpendicular;
/// and the pattern is generally rotated in the page. Treating the map as two
/// independent scale factors turns a circle into a visibly larger ellipse.
///
/// **Only `x` and `y` are used.** For [`crate::Projection::Cylindrical`],
/// `flat.edges_flat` carries the original `x` in its `z` slot, and nothing ever
/// draws it — measuring in 3D counts a component that never reaches the page
/// and oversizes every hole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatJacobian {
    pub m: [[f64; 2]; 2],
}

impl FlatJacobian {
    /// The `(du, dv)` that produces a given displacement on the page.
    ///
    /// Returns `(0, 0)` when the map is too ill-conditioned to invert
    /// meaningfully — a cell that collapses to a line or a point on the page has
    /// no `(du, dv)` that produces a displacement across it.
    ///
    /// The test is on the conditioning, not on the determinant alone. A
    /// determinant is only small or large *relative to the entries*: this
    /// Jacobian's are drawing units per surface unit, so on a millimetre pattern
    /// they run into the thousands and an absolute threshold means nothing. For
    /// a 2×2, `σ_min = |det| / σ_max`, so comparing `|det|` against the squared
    /// Frobenius norm bounds the condition number — here at `1e12`, which is
    /// twelve orders of magnitude clear of any cell a real pattern contains.
    ///
    /// Worth being strict about: dividing by a determinant of `8e-14` built from
    /// entries of order `10` once returned a displacement of `4e13` surface
    /// units, and the caller then walked the strips it spanned. See the plan's
    /// §8.8.
    pub fn solve(&self, dx: f64, dy: f64) -> (f64, f64) {
        let [[a, b], [c, d]] = self.m;
        let det = a * d - b * c;
        let frobenius_squared = a * a + b * b + c * c + d * d;
        if det.abs() <= 1e-12 * frobenius_squared {
            return (0.0, 0.0);
        }
        (((d * dx) - (b * dy)) / det, ((a * dy) - (c * dx)) / det)
    }
}

/// The local `(u, v)` to page map at a point.
///
/// Measured on the flat pattern rather than the surface: a hole is cut from
/// flat material, so its size and shape there are what matter.
///
/// Taken from the **containing triangle**, not the quad's bilinear average.
/// [`flat_point`] interpolates per triangle, and a quad's two triangles carry
/// different affine maps unless the quad is a parallelogram — which a tapered
/// gore never is. Averaging them matches neither, and oversizes a hole by ~9%
/// on the default shape. The triangle's own map is exact wherever it applies.
pub fn flat_jacobian(param: &SurfaceParam, flat: &FlatGeometry, u: f64, v: f64) -> FlatJacobian {
    jacobian_of(param, flat, param.cell(u, v))
}

fn jacobian_of(param: &SurfaceParam, flat: &FlatGeometry, cell: Cell) -> FlatJacobian {
    let c = flat_corners(flat, cell.ip, cell.it);

    // Corners in quad coordinates are v00 (0,0), v10 (1,0), v11 (1,1), v01 (0,1).
    // Read the two edges of whichever triangle applies.
    let (d_ds, d_dt) = match cell.triangle {
        // (v00, v10, v11): along s at t=0, then along t at s=1.
        0 => (c[1] - c[0], c[2] - c[1]),
        // (v00, v11, v01): along t at s=0, then along s at t=1.
        _ => (c[2] - c[3], c[3] - c[0]),
    };

    // s spans one strip, i.e. 1/phi_divisions of u; t spans one row of v.
    let d_du = d_ds * param.phi_divisions as f64;
    let d_dv = d_dt / param.row_v_span(cell.it).max(1e-12);

    FlatJacobian {
        m: [[d_du.x, d_dv.x], [d_du.y, d_dv.y]],
    }
}

/// Möller–Trumbore. Returns `(distance, b1, b2)` for a front- or back-facing hit.
fn ray_triangle(
    origin: DVec3,
    dir: DVec3,
    a: DVec3,
    b: DVec3,
    c: DVec3,
) -> Option<(f64, f64, f64)> {
    const EPS: f64 = 1e-12;
    let e1 = b - a;
    let e2 = c - a;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < EPS {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = origin - a;
    let u = tvec.dot(p) * inv;
    if !(-1e-9..=1.0 + 1e-9).contains(&u) {
        return None;
    }
    let q = tvec.cross(e1);
    let v = dir.dot(q) * inv;
    if v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    let t = e2.dot(q) * inv;
    if t < 0.0 { None } else { Some((t, u, v)) }
}

/// Intersect a ray with the surface, returning the nearest hit.
///
/// Brute force over every triangle. At realistic division counts that is a few
/// hundred tests per click — far cheaper than maintaining an acceleration
/// structure that would have to be rebuilt on every parameter change.
///
/// Both faces count as hits, so clicking works regardless of winding.
pub fn ray_hit(
    param: &SurfaceParam,
    geometry: &Geometry,
    origin: DVec3,
    direction: DVec3,
) -> Option<SurfaceHit> {
    let dir = direction.normalize_or_zero();
    if dir == DVec3::ZERO {
        return None;
    }

    let mut best: Option<(f64, usize, usize, usize, f64, f64)> = None;

    for ip in 0..geometry.phi_divisions {
        for it in 0..geometry.theta_divisions {
            let c = surface_corners(geometry, ip, it);
            for (triangle, (a, b, cc)) in [(c[0], c[1], c[2]), (c[0], c[2], c[3])]
                .into_iter()
                .enumerate()
            {
                if let Some((t, b1, b2)) = ray_triangle(origin, dir, a, b, cc)
                    && best.is_none_or(|(bt, ..)| t < bt)
                {
                    best = Some((t, ip, it, triangle, b1, b2));
                }
            }
        }
    }

    let (t, ip, it, triangle, b1, b2) = best?;

    // Invert the weights to recover the in-quad position.
    let (s, quad_t) = match triangle {
        0 => (b1 + b2, b2),
        _ => (b1, b1 + b2),
    };
    let (u, v) = param.coord(ip, it, s, quad_t);

    Some(SurfaceHit {
        u,
        v,
        point: origin + dir * t,
        distance: t,
    })
}

/// Barycentric coordinates of `p` in triangle `abc`, in the page plane.
fn barycentric_2d(p: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<[f64; 3]> {
    let (v0x, v0y) = (b.x - a.x, b.y - a.y);
    let (v1x, v1y) = (c.x - a.x, c.y - a.y);
    let (v2x, v2y) = (p.x - a.x, p.y - a.y);

    let det = v0x * v1y - v1x * v0y;
    if det.abs() < 1e-15 {
        return None;
    }
    let b1 = (v2x * v1y - v1x * v2y) / det;
    let b2 = (v0x * v2y - v2x * v0y) / det;
    Some([1.0 - b1 - b2, b1, b2])
}

/// The surface coordinate of a point on the flat pattern — the inverse of
/// [`flat_point`].
///
/// Brute force over every triangle, taking the one that contains the point. At
/// realistic division counts that is a few hundred tests, which is nothing for
/// a pointer query, and it needs no acceleration structure to invalidate when
/// the geometry changes.
///
/// Returns `None` when the point is off the pattern. Only `x` and `y` are read,
/// since those are what the page shows.
pub fn flat_to_surface(
    param: &SurfaceParam,
    flat: &FlatGeometry,
    page: DVec3,
) -> Option<(f64, f64)> {
    const EPS: f64 = 1e-9;

    for ip in 0..param.phi_divisions {
        for it in 0..param.theta_divisions {
            let c = flat_corners(flat, ip, it);
            for (triangle, (a, b, cc)) in [(c[0], c[1], c[2]), (c[0], c[2], c[3])]
                .into_iter()
                .enumerate()
            {
                let Some(w) = barycentric_2d(page, a, b, cc) else {
                    continue;
                };
                if w.iter().any(|x| *x < -EPS) {
                    continue;
                }
                // Same weight-to-quad-position inversion as `ray_hit`.
                let (s, t) = match triangle {
                    0 => (w[1] + w[2], w[2]),
                    _ => (w[1], w[1] + w[2]),
                };
                return Some(param.coord(ip, it, s, t));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::EllipsoidInput;
    use crate::{compute_flat_geometry, compute_geometry};

    fn setup() -> (SurfaceParam, Geometry, FlatGeometry) {
        let input = EllipsoidInput::default();
        let g = compute_geometry(&input);
        let f = compute_flat_geometry(&g, &input);
        let p = SurfaceParam::new(&g);
        (p, g, f)
    }

    #[test]
    fn cell_puts_corners_in_the_right_place() {
        let (p, ..) = setup();
        let cell = p.cell(0.0, 0.0);
        assert_eq!((cell.ip, cell.it), (0, 0));
        assert!((cell.weights[0] - 1.0).abs() < 1e-12, "{:?}", cell.weights);

        // v == 1.0 must land in the last cell, not one past it.
        assert_eq!(p.cell(0.999, 1.0).it, p.theta_divisions - 1);
    }

    #[test]
    fn cell_weights_always_sum_to_one() {
        let (p, ..) = setup();
        for i in 0..37 {
            for j in 0..37 {
                let (u, v) = (i as f64 / 36.0, j as f64 / 36.0);
                let w = p.cell(u, v).weights;
                assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12, "{w:?}");
                assert!(w.iter().all(|x| *x >= -1e-12), "negative weight {w:?}");
            }
        }
    }

    #[test]
    fn v_round_trips_through_the_arc_table() {
        let (p, ..) = setup();
        for i in 0..=40 {
            let v = i as f64 / 40.0;
            let back = p.v_of_index(p.theta_index(v));
            assert!((back - v).abs() < 1e-9, "{v} -> {back}");
        }
    }

    #[test]
    fn u_wraps_around_phi() {
        let (p, g, _) = setup();
        let a = surface_point(&p, &g, 0.25, 0.5);
        let b = surface_point(&p, &g, 1.25, 0.5);
        assert!((a - b).length() < 1e-9, "{a:?} vs {b:?}");
    }

    #[test]
    fn surface_coordinates_round_trip_through_a_ray() {
        let (p, g, _) = setup();

        for &(u, v) in &[(0.1, 0.2), (0.5, 0.5), (0.77, 0.9), (0.0, 0.35)] {
            let target = surface_point(&p, &g, u, v);
            let outward = (target - DVec3::new(0.0, 0.0, target.z)).normalize_or_zero();
            let origin = target + outward * 100.0;
            let hit = ray_hit(&p, &g, origin, -outward).expect("ray should hit the surface");

            assert!(
                (hit.point - target).length() < 1e-6,
                "point {:?} != {target:?}",
                hit.point
            );
            // Compare positions rather than (u, v): a ray may enter through a
            // coincident spot at a seam.
            let back = surface_point(&p, &g, hit.u, hit.v);
            assert!((back - target).length() < 1e-6, "{back:?} != {target:?}");
        }
    }

    #[test]
    fn a_ray_that_misses_returns_nothing() {
        let (p, g, _) = setup();
        let origin = DVec3::new(1000.0, 1000.0, 1000.0);
        assert!(ray_hit(&p, &g, origin, DVec3::new(1.0, 1.0, 1.0)).is_none());
    }

    #[test]
    fn flat_mapping_is_exact_at_every_shared_corner() {
        // Both mappings must resolve a grid corner to that corner exactly, which
        // is what makes the piecewise-affine transfer between them valid.
        let (p, g, f) = setup();
        for ip in 0..g.phi_divisions {
            for it in 0..=g.theta_divisions {
                let (u, v) = p.coord(ip, it, 0.0, 0.0);
                let got = flat_point(&p, &f, u, v);
                let want = f.edges_flat[ip][it][0];
                assert!(
                    (got - want).length() < 1e-9,
                    "[{ip}][{it}]: {got:?} != {want:?}"
                );
            }
        }
    }

    /// Drift between two tessellations, sampled over the whole surface.
    fn drift_between(a: &EllipsoidInput, b: &EllipsoidInput) -> f64 {
        let (ga, gb) = (compute_geometry(a), compute_geometry(b));
        let (pa, pb) = (SurfaceParam::new(&ga), SurfaceParam::new(&gb));

        let mut worst: f64 = 0.0;
        for i in 0..=12 {
            for j in 0..=12 {
                let (u, v) = (i as f64 / 12.0, j as f64 / 12.0);
                let pt_a = surface_point(&pa, &ga, u, v);
                let pt_b = surface_point(&pb, &gb, u, v);
                worst = worst.max((pt_a - pt_b).length());
            }
        }
        worst
    }

    #[test]
    fn arc_length_v_survives_a_change_of_theta_divisions() {
        // The reason `v` is arc length and not an index fraction: the equator,
        // h_middle, and h_bottom rows are a fixed *count*, so in index space
        // their share of the range shrinks as theta_divisions grows, sliding
        // everything above them. Holding phi fixed isolates that effect.
        let coarse = EllipsoidInput {
            phi_divisions: 8,
            theta_divisions: 16,
            ..Default::default()
        };
        let fine = EllipsoidInput {
            theta_divisions: 48,
            ..coarse.clone()
        };

        // What remains is only the theta polyline cutting corners off the true
        // profile, which is small.
        let worst = drift_between(&coarse, &fine);
        assert!(worst < 0.05, "worst drift {worst}");
    }

    #[test]
    fn changing_phi_divisions_moves_the_surface_itself() {
        // Not a parametrisation defect, and worth pinning down so it is not
        // mistaken for one: the surface is a polygon in phi, so a coarse ring
        // sits measurably inside a fine one. A hole keeps its place *on the
        // surface*; the surface is what moved.
        let coarse = EllipsoidInput {
            phi_divisions: 8,
            theta_divisions: 16,
            ..Default::default()
        };
        let fine = EllipsoidInput {
            phi_divisions: 24,
            ..coarse.clone()
        };

        // Roughly the sagitta of a 45-degree chord at the widest semi-axis:
        // 3.75 * (1 - cos(22.5 deg)) ~= 0.28.
        let worst = drift_between(&coarse, &fine);
        assert!((0.1..0.4).contains(&worst), "unexpected drift {worst}");
    }

    #[test]
    fn flat_to_surface_inverts_flat_point() {
        let (p, _, f) = setup();
        for &(u, v) in &[(0.1, 0.2), (0.3, 0.55), (0.66, 0.8), (0.95, 0.42)] {
            let page = flat_point(&p, &f, u, v);
            let (bu, bv) = flat_to_surface(&p, &f, page).expect("point is on the pattern");
            // Compare positions, not coordinates: distinct (u, v) can share a
            // page point at a seam, but the mapping must round-trip.
            let back = flat_point(&p, &f, bu, bv);
            assert!(
                (back.x - page.x).abs() < 1e-6 && (back.y - page.y).abs() < 1e-6,
                "({u}, {v}) -> {page:?} -> ({bu}, {bv}) -> {back:?}"
            );
        }
    }

    #[test]
    fn a_point_off_the_pattern_has_no_surface_coordinate() {
        let (p, _, f) = setup();
        assert!(flat_to_surface(&p, &f, DVec3::new(1e6, 1e6, 0.0)).is_none());
    }

    #[test]
    fn translating_a_polygon_moves_every_vertex_alike() {
        let mut shape = Cutout::polygon(vec![[0.2, 0.3], [0.3, 0.3], [0.3, 0.45]]);
        shape.translate(0.1, 0.05);
        let Cutout::Polygon { points } = &shape else {
            panic!("still a polygon")
        };
        for (got, want) in points.iter().zip([[0.3, 0.35], [0.4, 0.35], [0.4, 0.5]]) {
            assert!((got[0] - want[0]).abs() < 1e-12, "{points:?}");
            assert!((got[1] - want[1]).abs() < 1e-12, "{points:?}");
        }
    }

    #[test]
    fn translating_a_polygon_off_the_top_clamps_without_deforming() {
        // Clamping each vertex independently would squash the shape flat
        // against the edge; the whole move is limited instead.
        let mut shape = Cutout::polygon(vec![[0.2, 0.8], [0.3, 0.8], [0.3, 0.9]]);
        shape.translate(0.0, 0.5);
        let Cutout::Polygon { points } = &shape else {
            unreachable!()
        };
        assert!((points[2][1] - 1.0).abs() < 1e-12, "{points:?}");
        assert!((points[0][1] - 0.9).abs() < 1e-12, "{points:?}");
    }

    #[test]
    fn an_old_style_hole_still_deserialises() {
        let hole: Cutout = serde_json::from_str(r#"{"u":0.25,"v":0.5,"diameter":0.125}"#).unwrap();
        assert_eq!(hole, Cutout::hole(0.25, 0.5, 0.125));

        let shape: Cutout = serde_json::from_str(r#"{"points":[[0.0,0.0],[1.0,0.0],[1.0,1.0]]}"#)
            .expect("polygon form");
        assert!(matches!(shape, Cutout::Polygon { .. }));
    }

    #[test]
    fn default_diameter_is_three_millimetres() {
        assert!((Cutout::default_diameter(Unit::Mm) - 3.0).abs() < 1e-12);
        assert!((Cutout::default_diameter(Unit::Inch) - 3.0 / 25.4).abs() < 1e-6);
    }

    /// Dragging a shape over `u = 0` must move it, not tear it.
    ///
    /// `u` wraps, so a shape can legitimately straddle the seam. Wrapping each
    /// vertex on its own instead splits it across the whole pattern: this
    /// 0.08-wide square came out 0.92 wide, with its anchor thrown to the
    /// opposite side.
    #[test]
    fn a_shape_dragged_over_the_seam_keeps_its_shape() {
        let width = |c: &Cutout| match c {
            Cutout::Polygon { points } => {
                let (lo, hi) = points
                    .iter()
                    .fold((f64::MAX, f64::MIN), |(l, h), p| (l.min(p[0]), h.max(p[0])));
                hi - lo
            }
            _ => unreachable!(),
        };

        let mut shape = Cutout::polygon(vec![[0.90, 0.4], [0.98, 0.4], [0.98, 0.6], [0.90, 0.6]]);
        let before = width(&shape);

        // Nudge it over the seam in small steps, as a drag would. The mean
        // starts at 0.94, so it is astride the seam from step 4 to step 8.
        for step in 1..=12 {
            shape.translate(0.01, 0.0);
            assert!(
                (width(&shape) - before).abs() < 1e-12,
                "step {step}: width changed to {} at anchor {:?}",
                width(&shape),
                shape.anchor()
            );

            // While astride, the anchor has to stay at the seam. Averaging
            // wrapped vertices instead put it at 0.5 — half a turn away.
            let (au, _) = shape.anchor();
            let expected = (0.94 + 0.01 * step as f64).rem_euclid(1.0);
            let apart = (au - expected).abs().min(1.0 - (au - expected).abs());
            assert!(
                apart < 1e-9,
                "step {step}: anchor {au}, expected {expected}"
            );
        }
    }

    /// A cell that collapses on the page has no inverse worth having. Plan §8.8.
    #[test]
    fn an_edge_on_cell_refuses_to_be_inverted() {
        // What three strips actually produced: a `∂/∂u` column of numerical
        // noise beside a healthy `∂/∂v`. Its determinant is 8e-14 — far above
        // any absolute epsilon, and meaningless all the same.
        let collapsed = FlatJacobian {
            m: [[8.0e-15, 0.0], [-1.2e-15, 10.129]],
        };
        assert_eq!(collapsed.solve(0.35, 0.0), (0.0, 0.0));

        // A real cell from the same pattern, inverted as usual.
        let healthy = FlatJacobian {
            m: [[10.9539, 3.9677], [0.0, 10.4187]],
        };
        let (du, dv) = healthy.solve(0.35, 0.0);
        assert!(du.abs() < 1.0 && dv.abs() < 1.0, "({du}, {dv})");
        let back = (
            healthy.m[0][0] * du + healthy.m[0][1] * dv,
            healthy.m[1][0] * du + healthy.m[1][1] * dv,
        );
        assert!(
            (back.0 - 0.35).abs() < 1e-12 && back.1.abs() < 1e-12,
            "{back:?}"
        );

        // Scale-free: the same shape of cell in millimetres, where every entry
        // is 25x larger, must be treated the same way.
        let millimetres = FlatJacobian {
            m: [[278.2, 100.8], [0.0, 264.6]],
        };
        assert_ne!(millimetres.solve(8.9, 0.0), (0.0, 0.0));
    }

    /// And going all the way round returns it to where it started.
    #[test]
    fn a_shape_taken_right_around_comes_back() {
        let start = vec![[0.30, 0.4], [0.38, 0.4], [0.38, 0.6], [0.30, 0.6]];
        let mut shape = Cutout::polygon(start.clone());
        for _ in 0..100 {
            shape.translate(0.01, 0.0);
        }
        let Cutout::Polygon { points } = &shape else {
            unreachable!()
        };
        for (got, want) in points.iter().zip(&start) {
            assert!(
                (got[0] - want[0]).abs() < 1e-9 && (got[1] - want[1]).abs() < 1e-9,
                "{got:?} vs {want:?}"
            );
        }
    }
}
