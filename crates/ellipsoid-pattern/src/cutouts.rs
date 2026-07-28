//! Splitting holes across panel seams.
//!
//! Panel strips are cut as separate pieces, so a hole straddling a seam has to
//! be divided between them. Drawn whole it would span a cut, and the two halves
//! would not line up once the panels were joined.
//!
//! # Clipping in surface space, not flat space
//!
//! `RUST_CONVERSION_PLAN.md` §7.3 anticipated needing a 2D boolean library
//! (`i_overlay`) for this, on the grounds that a flattened panel strip is
//! concave and Sutherland–Hodgman only clips against convex regions.
//!
//! That is true in flat space — but in **surface** coordinates strip `ip` is
//! simply the band `u ∈ [ip/N, (ip+1)/N]`, which is a rectangle. Clipping a
//! polygon to a rectangle is four half-plane passes and needs no dependency.
//! The strip boundaries come out exact rather than polygonised, and mapping the
//! result back through [`flat_point`] reuses the same piecewise-affine map that
//! places the holes in the first place.
//!
//! The end caps (`v ∈ [0, 1]`) are clipped too, so a hole near the top or
//! bottom edge cannot spill off the pattern.

use ellipsoid_core::DVec3;
use ellipsoid_core::flatten::FlatGeometry;
use ellipsoid_core::geometry::Geometry;
use ellipsoid_core::surface::{Cutout, SurfaceParam, flat_jacobian_in_strip, flat_point_in_strip};
use glam::DVec2;

/// Rim resolution. Fine enough to read as a circle, coarse enough that a
/// pattern full of holes stays a manageable file.
pub const RIM_SEGMENTS: usize = 64;

/// Newton steps per rim point. The map is piecewise-affine, so the first
/// correction lands almost exactly and a third is already redundant.
const REFINEMENTS: usize = 3;

/// Which strip a `u` belongs to, as a signed index so a shape may run off
/// either end of the phi range.
fn strip_of(u: f64, phi_divisions: usize) -> i64 {
    (u * phi_divisions as f64).floor() as i64
}

/// A hole's rim in `(u, v)`, shaped so it comes out circular on the page.
///
/// Two stages. The local Jacobian gives the starting guess — solving through it
/// rather than dividing by two scale factors is what keeps a circle a circle,
/// since the map both shears and rotates.
///
/// That guess is then refined against the real mapping, because the Jacobian is
/// only exact inside the triangle it was taken from: a hole spans neighbouring
/// triangles whose maps differ, leaving a ~3% size error on a 3 mm hole and
/// ~15% on one as large as a panel cell.
///
/// Both stages work in the **home strip's** frame, extended past its edges.
/// That extension is continuous where the true map is not, so refinement cannot
/// fall off a seam and needs no divergence guard. The rim is therefore a true
/// circle in the geometry of the panel the hole was placed on, which is exactly
/// the shape the two halves must share for them to meet when the panels are
/// joined.
fn rim(param: &SurfaceParam, flat: &FlatGeometry, cutout: &Cutout, home: i64) -> Vec<DVec2> {
    let (cu, cv, diameter) = match cutout {
        Cutout::Hole { u, v, diameter } => (*u, *v, *diameter),
        // A polygon is already given in surface coordinates, so it needs no
        // fitting — the vertices are the outline.
        Cutout::Polygon { points } => {
            return points.iter().map(|p| DVec2::new(p[0], p[1])).collect();
        }
    };

    let jacobian = flat_jacobian_in_strip(param, flat, home, cu, cv);
    let radius = diameter / 2.0;
    let page = |p: DVec3| DVec2::new(p.x, p.y);
    let at = |u: f64, v: f64| page(flat_point_in_strip(param, flat, home, u, v));
    let center = at(cu, cv);

    (0..RIM_SEGMENTS)
        .map(|i| {
            let angle = i as f64 / RIM_SEGMENTS as f64 * std::f64::consts::TAU;
            let target = DVec2::new(radius * angle.cos(), radius * angle.sin());
            let error_at = |du: f64, dv: f64| target - (at(cu + du, cv + dv) - center);

            let (mut du, mut dv) = jacobian.solve(target.x, target.y);
            for _ in 0..REFINEMENTS {
                let error = error_at(du, dv);
                let (ddu, ddv) = jacobian.solve(error.x, error.y);
                du += ddu;
                dv += ddv;
            }
            DVec2::new(cu + du, cv + dv)
        })
        .collect()
}

/// A cutout's outline in surface coordinates, fitted in its home strip.
///
/// The 3D views cut the same shape out of the surface mesh that the pattern
/// cuts out of the page; going through this one rim is what keeps the preview
/// and the pattern showing the same hole.
pub fn rim_uv(param: &SurfaceParam, flat: &FlatGeometry, cutout: &Cutout) -> Vec<DVec2> {
    let home = strip_of(cutout.anchor().0, param.phi_divisions);
    rim(param, flat, cutout, home)
}

/// Insert a vertex wherever an edge crosses a grid cell boundary.
///
/// The map from `(u, v)` to the page is affine *within* a cell and bends at
/// every boundary. An edge carried by its two endpoints alone is therefore
/// mapped as a straight chord across something that bends, and the material
/// between chord and curve survives the subtraction.
///
/// **This has to run after clipping, not before.** The edge that matters most
/// is the one clipping *creates*: a shape crossing a seam gets a new side along
/// `u = k/N`, running from wherever the outline entered the strip to wherever it
/// left, with nothing in between. On this pattern that side is a chord across a
/// seam which bows out by 0.6 in where the panels join, so a shape drawn over a
/// seam left a strip of panel down the middle of it that wide.
///
/// A hole's rim never showed the effect: sixty-four segments are short enough
/// that it disappears. A hand-drawn shape has a handful of long ones.
///
/// Once every edge lies inside one cell the piecewise-affine map reproduces it
/// exactly, so this is a correction and not an approximation that could be
/// refined further.
fn densify(param: &SurfaceParam, outline: &[DVec2]) -> Vec<DVec2> {
    if outline.len() < 2 {
        return outline.to_vec();
    }

    // `v` of each theta row boundary. Not evenly spaced — `v` is arc length.
    let rows: Vec<f64> = (0..=param.theta_divisions)
        .map(|it| param.coord(0, it, 0.0, 0.0).1)
        .collect();
    let strips = param.phi_divisions as f64;

    let mut out = Vec::with_capacity(outline.len() * 4);
    for i in 0..outline.len() {
        let a = outline[i];
        let b = outline[(i + 1) % outline.len()];
        out.push(a);

        let mut cuts: Vec<f64> = Vec::new();
        let mut crossings = |from: f64, to: f64, boundaries: &dyn Fn(&mut Vec<f64>)| {
            if (to - from).abs() > f64::EPSILON {
                boundaries(&mut cuts);
            }
        };

        // Strip boundaries. The range is taken from the edge itself so a shape
        // sitting off either end of the phi range is still handled.
        crossings(a.x, b.x, &|cuts: &mut Vec<f64>| {
            let (lo, hi) = if a.x < b.x { (a.x, b.x) } else { (b.x, a.x) };
            let first = (lo * strips).floor() as i64;
            let last = (hi * strips).ceil() as i64;
            // An edge spanning more than a couple of turns is degenerate — a
            // rim fit that diverged, most likely. Subdividing it boundary by
            // boundary would be a very long loop to no purpose, so leave it
            // alone and let the clip discard it.
            if last.saturating_sub(first) > 4 * strips as i64 {
                return;
            }
            for k in first..=last {
                cuts.push((k as f64 / strips - a.x) / (b.x - a.x));
            }
        });
        // Theta row boundaries.
        crossings(a.y, b.y, &|cuts: &mut Vec<f64>| {
            for row in &rows {
                cuts.push((row - a.y) / (b.y - a.y));
            }
        });

        cuts.retain(|t| *t > 1e-12 && *t < 1.0 - 1e-12);
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
        for t in cuts {
            out.push(a + (b - a) * t);
        }
    }
    out
}

/// Area enclosed by a closed ring, unsigned.
pub fn area(points: &[DVec2]) -> f64 {
    signed_area(points).abs() / 2.0
}

/// Remove hairline spikes: vertices whose two neighbours all but coincide.
///
/// A cutout that crosses a seam is subtracted as one piece per strip, and each
/// piece is placed with *its own* strip pinned — which is the only way to place
/// it, but it means the two pieces meet along an edge the two strips do not
/// flatten to quite the same place. The gap is a fraction of a millimetre, and
/// where the boolean welds the pieces anyway it leaves the discrepancy behind as
/// a needle of material standing inside the hole:
///
/// ```text
///   (346.594, 317.420)      the two sides of one seam, 0.2 apart...
///   (346.770, 309.507)      ...with the ring running 7.9 up between them
///   (346.819, 317.422)      and straight back down
/// ```
///
/// Nothing downstream can see this. The ring is closed and simple, its area is
/// unremarkable, and the spike is a feature *within* it rather than a ring of
/// its own — so the "too small to be worth cutting" filter, which works on whole
/// rings, passes it straight through. It reaches the drawing as a stray tick.
///
/// The cut outline has always had the same problem and solves it the same way:
/// two panel edges closer than `min_gap` are treated as one edge and the
/// duplicate points dropped. This is that rule, applied to what the boolean
/// gives back.
///
/// A spike is recognised by the ring coming back to where it was: two vertices
/// within `tolerance` of each other, with a path between them *longer* than
/// that. On a smooth curve those two conditions never hold together — points a
/// stroke width apart are joined by a path a stroke width long — so a densely
/// sampled rim passes through untouched. Only a there-and-back does both.
///
/// The tip of a spike is not always a vertex. Welded from two sides, it is
/// usually a very short *edge* between the two, which is why the span is
/// searched rather than each vertex examined in isolation:
///
/// ```text
///   (346.289, 320.908)   on the hole's bottom edge
///   (346.594, 317.420)   up
///   (346.819, 317.422)   across 0.2 — the tip
///   (346.903, 320.922)   and back down to the bottom edge
/// ```
///
/// Iterative because collapsing one exposes the next: the five-point excursion
/// this started as becomes a four-point one, then nothing.
pub fn despike(ring: &mut Vec<DVec2>, tolerance: f64) {
    /// How many vertices a spike may be made of. Two is enough for a welded tip
    /// and keeps this from reaching across a real feature to find its far side.
    const LONGEST_SPIKE: usize = 3;

    if tolerance <= 0.0 {
        return;
    }

    // A triangle is the smallest thing that still encloses anything. Anything
    // reduced that far is left for the area filter to discard, rather than
    // being flattened into a line here.
    while ring.len() > 3 {
        let n = ring.len();
        let mut collapsed = false;

        'search: for start in 0..n {
            let mut walked = 0.0;
            for span in 1..=LONGEST_SPIKE.min(n - 2) {
                walked += ring[(start + span - 1) % n].distance(ring[(start + span) % n]);
                if span < 2 {
                    continue;
                }
                let end = (start + span) % n;
                if ring[start].distance(ring[end]) <= tolerance && walked > tolerance {
                    let doomed: Vec<usize> = (1..span).map(|k| (start + k) % n).collect();
                    let mut kept = Vec::with_capacity(n - doomed.len());
                    for (index, point) in ring.iter().enumerate() {
                        if !doomed.contains(&index) {
                            kept.push(*point);
                        }
                    }
                    *ring = kept;
                    collapsed = true;
                    break 'search;
                }
            }
        }

        if !collapsed {
            return;
        }
    }
}

/// Twice the signed area. Positive means counter-clockwise.
fn signed_area(points: &[DVec2]) -> f64 {
    let mut sum = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    sum
}

/// Force counter-clockwise winding.
///
/// The boolean below needs consistent orientation across every input contour.
/// Rim points are generated counter-clockwise in `(u, v)`, but the map to the
/// page can flip them, and the cut outline's own direction depends on the
/// projection.
pub(crate) fn make_ccw(mut points: Vec<DVec2>) -> Vec<DVec2> {
    if signed_area(&points) < 0.0 {
        points.reverse();
    }
    points
}

/// The cut outline with cutouts removed from it.
///
/// Returns `(outer, holes)`: outlines to cut as the panel boundary, and closed
/// rings to cut inside it.
///
/// A cutout wholly inside a panel comes back as a hole ring, exactly as if it
/// had been drawn separately. One that reaches a panel edge instead *opens that
/// edge up* — the boundary detours around it and no chord is cut across the
/// seam. That is what makes the two halves of a split shape meet: each panel
/// carries a notch, and joining the panels closes the shape.
pub fn subtract_from_outline(
    outline: &[DVec2],
    pieces: &[Vec<DVec2>],
) -> (Vec<Vec<DVec2>>, Vec<Vec<DVec2>>) {
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::core::overlay_rule::OverlayRule;
    use i_overlay::float::single::SingleFloatOverlay;

    if outline.len() < 3 || pieces.is_empty() {
        return (vec![outline.to_vec()], Vec::new());
    }

    let to_pairs = |pts: &[DVec2]| -> Vec<[f64; 2]> { pts.iter().map(|p| [p.x, p.y]).collect() };
    let subject = vec![to_pairs(&make_ccw(outline.to_vec()))];
    let clip: Vec<Vec<[f64; 2]>> = pieces
        .iter()
        .filter(|p| p.len() >= 3)
        .map(|p| to_pairs(&make_ccw(p.clone())))
        .collect();

    if clip.is_empty() {
        return (vec![outline.to_vec()], Vec::new());
    }

    let shapes = subject.overlay(&clip, OverlayRule::Difference, FillRule::NonZero);

    let mut outer = Vec::new();
    let mut holes = Vec::new();
    for shape in shapes {
        // Contour 0 is the outer boundary; the rest are its holes.
        for (index, contour) in shape.into_iter().enumerate() {
            let ring: Vec<DVec2> = contour
                .into_iter()
                .map(|p| DVec2::new(p[0], p[1]))
                .collect();
            if ring.len() < 3 {
                continue;
            }
            if index == 0 {
                outer.push(ring)
            } else {
                holes.push(ring)
            }
        }
    }

    // A degenerate result would silently erase the pattern; keep the original.
    if outer.is_empty() {
        return (vec![outline.to_vec()], Vec::new());
    }
    (outer, holes)
}

/// Which side of an axis-aligned boundary a point is on.
#[derive(Clone, Copy)]
enum Edge {
    MinX(f64),
    MaxX(f64),
    MinY(f64),
    MaxY(f64),
}

impl Edge {
    fn inside(self, p: DVec2) -> bool {
        match self {
            Edge::MinX(v) => p.x >= v,
            Edge::MaxX(v) => p.x <= v,
            Edge::MinY(v) => p.y >= v,
            Edge::MaxY(v) => p.y <= v,
        }
    }

    /// Where the segment `a -> b` crosses this boundary.
    fn intersect(self, a: DVec2, b: DVec2) -> DVec2 {
        let t = match self {
            Edge::MinX(v) | Edge::MaxX(v) => (v - a.x) / (b.x - a.x),
            Edge::MinY(v) | Edge::MaxY(v) => (v - a.y) / (b.y - a.y),
        };
        // A boundary is only tested when the endpoints straddle it, so the
        // denominator cannot be zero; clamp anyway against rounding.
        a + (b - a) * t.clamp(0.0, 1.0)
    }
}

/// Sutherland–Hodgman against one half-plane.
fn clip_edge(polygon: &[DVec2], edge: Edge) -> Vec<DVec2> {
    if polygon.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(polygon.len() + 4);
    let mut previous = polygon[polygon.len() - 1];

    for &current in polygon {
        let (in_current, in_previous) = (edge.inside(current), edge.inside(previous));
        if in_current {
            if !in_previous {
                out.push(edge.intersect(previous, current));
            }
            out.push(current);
        } else if in_previous {
            out.push(edge.intersect(previous, current));
        }
        previous = current;
    }
    out
}

/// Clip a polygon to an axis-aligned rectangle.
fn clip_to_rect(polygon: &[DVec2], min: DVec2, max: DVec2) -> Vec<DVec2> {
    let mut result = polygon.to_vec();
    for edge in [
        Edge::MinX(min.x),
        Edge::MaxX(max.x),
        Edge::MinY(min.y),
        Edge::MaxY(max.y),
    ] {
        result = clip_edge(&result, edge);
        if result.is_empty() {
            break;
        }
    }
    result
}

/// The flat-space outline(s) of one hole: one per panel strip it touches.
///
/// A hole in the middle of a strip yields a single piece. One over a seam
/// yields two, each belonging to a different cut piece.
///
/// # A shape can still be split where the outline shows no seam
///
/// `draw_edges` draws two panels as one piece when the gap between them is no
/// more than `min_gap`, but the pieces here are always divided at the strip
/// boundary. A shape crossing such a seam therefore comes back as two holes a
/// hair apart, leaving a sliver of material across the middle of what looks
/// like solid panel. See `RUST_CONVERSION_PLAN.md` Appendix O.
pub fn pieces(
    param: &SurfaceParam,
    _geometry: &Geometry,
    flat: &FlatGeometry,
    cutout: &Cutout,
) -> Vec<Vec<DVec3>> {
    let home = strip_of(cutout.anchor().0, param.phi_divisions);
    let outline = rim(param, flat, cutout, home);
    if outline.is_empty() {
        return Vec::new();
    }

    let strips = param.phi_divisions as f64;
    let (min_u, max_u) = outline.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.x), hi.max(p.x))
    });

    // Only the strips the hole actually overlaps. The range may run below 0 or
    // past the last strip when a hole sits on the phi wrap; the strip index is
    // signed and wrapped only when indexing the geometry.
    let first = strip_of(min_u, param.phi_divisions);
    let last = strip_of(max_u, param.phi_divisions);

    // A shape cannot legitimately reach past one turn either side of the
    // pattern, so anything wider than three turns is a fit that diverged rather
    // than a hole. Walking its strips one at a time would take longer than
    // anyone will wait — a rim that once came back spanning 1e14 in `u` asked
    // for 3e14 iterations of the clip below, which reads from the outside
    // exactly like the app hanging. `densify` guards its own loop the same way.
    let reach = 3 * param.phi_divisions as i64;
    if last.saturating_sub(first) > reach {
        return Vec::new();
    }

    let mut out = Vec::new();
    for strip in first..=last {
        let lo = strip as f64 / strips;
        let hi = (strip + 1) as f64 / strips;
        let clipped = clip_to_rect(&outline, DVec2::new(lo, 0.0), DVec2::new(hi, 1.0));

        // Fewer than three points is a tangential graze, not a piece.
        if clipped.len() < 3 {
            continue;
        }
        // Now, with the cut edges in place — see [`densify`].
        let clipped = densify(param, &clipped);

        // Each piece is placed with *its own* strip pinned. Deriving the strip
        // from `u` instead would put points sitting exactly on the cut edge —
        // every clip intersection — on the neighbouring panel, and at the phi
        // wrap that neighbour is at the opposite end of the page.
        out.push(
            clipped
                .iter()
                .map(|p| flat_point_in_strip(param, flat, strip, p.x, p.y))
                .collect(),
        );
    }

    out
}

/// Whether `p` is inside the closed ring `polygon`.
///
/// Ray casting, so it handles the concave outlines a drawn shape can have.
fn contains(polygon: &[DVec2], p: DVec2) -> bool {
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if x > p.x {
                inside = !inside;
            }
        }
    }
    inside
}

fn cross(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}

/// Where `a -> b` crosses `c -> d`, as a fraction along `a -> b`.
fn segment_crossing(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Option<f64> {
    let (r, s) = (b - a, d - c);
    let denom = cross(r, s);
    if denom.abs() < 1e-12 {
        return None;
    }
    let t = cross(c - a, s) / denom;
    let u = cross(c - a, r) / denom;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

/// The parts of segment `a -> b` that lie outside every one of `pieces`.
///
/// Guide lines are fold and glue marks. Running one across a hole marks
/// material that is not there, and on a cutter that draws its guides it is a
/// stray line through the middle of the cutout — so the segments are split at
/// every crossing and only the parts still on panel are kept.
pub fn outside_pieces(a: DVec2, b: DVec2, pieces: &[Vec<DVec2>]) -> Vec<[DVec2; 2]> {
    if pieces.is_empty() {
        return vec![[a, b]];
    }

    let mut cuts = vec![0.0, 1.0];
    for piece in pieces {
        for i in 0..piece.len() {
            let (c, d) = (piece[i], piece[(i + 1) % piece.len()]);
            if let Some(t) = segment_crossing(a, b, c, d) {
                cuts.push(t);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);

    let at = |t: f64| a + (b - a) * t;
    cuts.windows(2)
        .filter(|w| {
            let mid = at((w[0] + w[1]) / 2.0);
            !pieces.iter().any(|p| contains(p, mid))
        })
        .map(|w| [at(w[0]), at(w[1])])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipsoid_core::{EllipsoidInput, compute_flat_geometry, compute_geometry};

    fn setup(input: &EllipsoidInput) -> (SurfaceParam, Geometry, FlatGeometry) {
        let g = compute_geometry(input);
        let f = compute_flat_geometry(&g, input);
        let p = SurfaceParam::new(&g);
        (p, g, f)
    }

    /// Longest distance between any two points of a piece, measured **on the
    /// page**.
    ///
    /// Only x and y: cylindrical output stashes the original x in z, and that
    /// component is never drawn.
    fn extent(piece: &[DVec3]) -> f64 {
        let mut worst: f64 = 0.0;
        for a in piece {
            for b in piece {
                worst = worst.max(DVec2::new(a.x - b.x, a.y - b.y).length());
            }
        }
        worst
    }

    #[test]
    fn a_hole_inside_one_strip_stays_whole() {
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);
        // Mid-strip, mid-height.
        let cutout = Cutout::hole(0.5 / 8.0, 0.5, 0.25);
        let pieces = pieces(&p, &g, &f, &cutout);
        assert_eq!(pieces.len(), 1, "should not have been split");
        assert_eq!(pieces[0].len(), RIM_SEGMENTS, "rim was altered");
    }

    #[test]
    fn a_hole_on_a_seam_splits_in_two() {
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);
        // Exactly on the seam between strips 1 and 2.
        let cutout = Cutout::hole(2.0 / 8.0, 0.5, 0.25);
        let pieces = pieces(&p, &g, &f, &cutout);
        assert_eq!(pieces.len(), 2, "seam hole was not split");
        for piece in &pieces {
            assert!(piece.len() >= 3, "degenerate piece");
        }
    }

    #[test]
    fn split_pieces_together_match_the_whole_hole() {
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);
        let diameter = 0.25;

        let whole = pieces(&p, &g, &f, &Cutout::hole(0.5 / 8.0, 0.5, diameter));
        let split = pieces(&p, &g, &f, &Cutout::hole(2.0 / 8.0, 0.5, diameter));

        // Two halves of the same hole: neither piece alone spans the full
        // diameter, but together they account for it.
        let whole_extent = extent(&whole[0]);
        assert!(
            split.iter().all(|piece| extent(piece) < whole_extent),
            "a split piece is as large as the whole hole"
        );
        let combined: Vec<DVec3> = split.concat();
        assert!(
            (extent(&combined) - whole_extent).abs() < whole_extent * 0.1,
            "halves span {} against {whole_extent} whole",
            extent(&combined)
        );
    }

    #[test]
    fn a_realistic_hole_is_the_size_it_was_asked_for() {
        // Sizing goes through the local Jacobian rather than being drawn as a
        // circle in flat space, so the physical size has to be checked.
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);
        let diameter = Cutout::default_diameter(ellipsoid_core::Unit::Inch); // 3 mm

        for v in [0.2, 0.4, 0.6, 0.8] {
            let cutout = Cutout::hole(0.5 / 8.0, v, diameter);
            let pieces = pieces(&p, &g, &f, &cutout);
            assert_eq!(pieces.len(), 1);
            let measured = extent(&pieces[0]);
            assert!(
                (measured - diameter).abs() < diameter * 0.02,
                "v={v} asked {diameter}, measured {measured}"
            );
        }
    }

    #[test]
    fn even_a_hole_as_big_as_a_panel_cell_stays_close() {
        // Refinement is what buys this. On the raw affine estimate these came
        // out up to 15% oversize, because a hole this large reaches triangles
        // whose maps differ sharply on a tapered gore.
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);

        let mut worst: f64 = 0.0;
        for diameter in [0.25, 0.5] {
            for v in [0.25, 0.5, 0.75] {
                let cutout = Cutout::hole(0.5 / 8.0, v, diameter);
                let measured = extent(&pieces(&p, &g, &f, &cutout)[0]);
                worst = worst.max((measured - diameter).abs() / diameter);
            }
        }
        assert!(worst < 0.02, "worst relative size error {worst}");
    }

    #[test]
    fn a_hole_across_the_phi_wrap_still_splits() {
        // u = 0 is a seam like any other, but the strips on either side are the
        // last and the first.
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);
        let cutout = Cutout::hole(0.0, 0.5, 0.25);
        let pieces = pieces(&p, &g, &f, &cutout);
        assert_eq!(pieces.len(), 2, "wrap-around hole was not split");
    }

    #[test]
    fn a_hole_at_the_bottom_edge_is_trimmed_not_dropped() {
        // The bottom edge, where `h_bottom` gives a full-width ring, so the
        // hole stays inside one strip and only the v clip applies.
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);
        let cutout = Cutout::hole(0.5 / 8.0, 0.0, 0.25);
        let pieces = pieces(&p, &g, &f, &cutout);
        assert_eq!(pieces.len(), 1, "edge hole vanished");
        // Half the rim is past the edge, so the piece is smaller than the rim.
        assert!(
            pieces[0].len() < RIM_SEGMENTS,
            "nothing was trimmed at the edge"
        );
    }

    #[test]
    fn a_hole_near_the_apex_spans_several_gores() {
        // Not a defect: gores converge to points at the top, so a hole of fixed
        // physical size genuinely covers a large share of the shrinking
        // circumference there. Recorded so the behaviour is deliberate.
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);
        let pieces = pieces(&p, &g, &f, &Cutout::hole(0.5 / 8.0, 1.0, 0.25));
        assert!(
            pieces.len() > 1,
            "expected an apex hole to cross gores, got {}",
            pieces.len()
        );
    }

    /// No piece may contain a jump: a hole's outline is a circle cut by at most
    /// one chord, so no edge of a piece can exceed the hole's own diameter.
    ///
    /// A longer edge means consecutive rim points landed in unrelated places —
    /// the "hook" artifact.
    #[test]
    fn no_piece_contains_a_jump() {
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);

        let mut worst: Option<(f64, f64, f64, f64)> = None;
        for iu in 0..64 {
            // Stops short of the apex, where gores converge to points and a
            // hole this size is wider than the gore itself; see
            // `a_hole_near_the_apex_spans_several_gores`.
            for iv in 1..29 {
                let (u, v) = (iu as f64 / 64.0, iv as f64 / 32.0);
                let diameter = 0.35;
                let cutout = Cutout::hole(u, v, diameter);
                for piece in pieces(&p, &g, &f, &cutout) {
                    for w in 0..piece.len() {
                        let a = piece[w];
                        let b = piece[(w + 1) % piece.len()];
                        let edge = DVec2::new(a.x - b.x, a.y - b.y).length();
                        if worst.is_none_or(|(e, ..)| edge > e) {
                            worst = Some((edge, u, v, diameter));
                        }
                    }
                }
            }
        }

        let (edge, u, v, diameter) = worst.expect("some pieces");
        assert!(
            edge <= diameter * 1.05,
            "jump of {edge} in a hole of {diameter} at u={u} v={v}"
        );
    }

    #[test]
    fn clipping_a_square_to_a_band_keeps_the_overlap() {
        let square = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(2.0, 0.0),
            DVec2::new(2.0, 2.0),
            DVec2::new(0.0, 2.0),
        ];
        let clipped = clip_to_rect(&square, DVec2::new(1.0, -1.0), DVec2::new(3.0, 3.0));
        assert!(clipped.iter().all(|p| p.x >= 1.0 - 1e-12), "{clipped:?}");
        assert!(
            clipped.iter().any(|p| (p.x - 1.0).abs() < 1e-12),
            "no points on the cut edge: {clipped:?}"
        );
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<DVec2> {
        vec![
            DVec2::new(x0, y0),
            DVec2::new(x1, y0),
            DVec2::new(x1, y1),
            DVec2::new(x0, y1),
        ]
    }

    #[test]
    fn a_cutout_inside_the_outline_becomes_a_hole_ring() {
        let outline = square(0.0, 0.0, 10.0, 10.0);
        let (outer, holes) = subtract_from_outline(&outline, &[square(4.0, 4.0, 6.0, 6.0)]);
        assert_eq!(outer.len(), 1, "outline should stay a single piece");
        assert_eq!(holes.len(), 1, "the cutout should become a ring");
    }

    #[test]
    fn a_cutout_over_the_edge_notches_the_outline_instead() {
        // The point of the whole exercise: no chord is cut across the edge, so
        // the boundary simply detours around the shape.
        let outline = square(0.0, 0.0, 10.0, 10.0);
        let (outer, holes) = subtract_from_outline(&outline, &[square(-1.0, 4.0, 2.0, 6.0)]);
        assert_eq!(outer.len(), 1);
        assert!(holes.is_empty(), "an edge cutout must not become a ring");
        assert!(
            outer[0].len() > outline.len(),
            "the outline gained no detour: {} points",
            outer[0].len()
        );
        // The notch reaches inward to the cutout's inner edge.
        let deepest = outer[0]
            .iter()
            .filter(|p| p.x > 0.5)
            .map(|p| p.x)
            .fold(f64::MAX, f64::min);
        assert!((deepest - 2.0).abs() < 1e-6, "notch depth {deepest}");
    }

    #[test]
    fn winding_direction_does_not_matter() {
        // Rim points come out clockwise or anticlockwise depending on the
        // projection and the page flip, so the boolean must not care.
        let outline = square(0.0, 0.0, 10.0, 10.0);
        let mut reversed = square(4.0, 4.0, 6.0, 6.0);
        reversed.reverse();

        let (_, forward) = subtract_from_outline(&outline, &[square(4.0, 4.0, 6.0, 6.0)]);
        let (_, backward) = subtract_from_outline(&outline, &[reversed]);
        assert_eq!(forward.len(), backward.len(), "winding changed the result");
        assert_eq!(forward.len(), 1);
    }

    #[test]
    fn an_outline_with_no_cutouts_is_returned_untouched() {
        let outline = square(0.0, 0.0, 10.0, 10.0);
        let (outer, holes) = subtract_from_outline(&outline, &[]);
        assert_eq!(outer, vec![outline]);
        assert!(holes.is_empty());
    }

    #[test]
    fn a_cutout_swallowing_the_outline_leaves_the_pattern_intact() {
        // Difference would legitimately return nothing here. Erasing the whole
        // pattern is never the useful answer, so the original is kept.
        let outline = square(0.0, 0.0, 10.0, 10.0);
        let (outer, _) = subtract_from_outline(&outline, &[square(-5.0, -5.0, 15.0, 15.0)]);
        assert_eq!(outer, vec![outline]);
    }

    #[test]
    fn clipping_something_entirely_outside_yields_nothing() {
        let square = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let clipped = clip_to_rect(&square, DVec2::new(5.0, 5.0), DVec2::new(6.0, 6.0));
        assert!(clipped.is_empty(), "{clipped:?}");
    }

    /// The needle a welded seam leaves inside a hole. Plan §8.10.
    #[test]
    fn a_hairline_spike_is_removed() {
        // The real thing, lifted out of a pattern: the bottom edge of a hole,
        // interrupted by an excursion 3.5 up, 0.2 across the tip, and back.
        let mut ring = vec![
            DVec2::new(300.0, 320.9),
            DVec2::new(346.289, 320.908),
            DVec2::new(346.594, 317.420),
            DVec2::new(346.819, 317.422),
            DVec2::new(346.903, 320.922),
            DVec2::new(400.0, 320.9),
            DVec2::new(400.0, 250.0),
            DVec2::new(300.0, 250.0),
        ];
        let before = area(&ring);
        despike(&mut ring, 1.26);

        assert!(
            !ring.iter().any(|p| p.y < 320.0 && p.y > 260.0),
            "the excursion is still there: {ring:?}"
        );
        // The hole gains the sliver of material that was standing in it, and
        // nothing else: a tenth of a percent of a hole this size.
        assert!(
            (area(&ring) - before).abs() < before * 0.001,
            "area moved from {before} to {}",
            area(&ring)
        );
    }

    /// The guard that matters: this must not quietly smooth real geometry.
    ///
    /// A rim is sampled far more finely than one stroke width in places, and a
    /// rule that collapsed any two nearby points would turn a circle into a
    /// polygon without anyone noticing.
    #[test]
    fn a_finely_sampled_curve_is_left_alone() {
        // 200 points around a circle of radius 30: neighbours are 0.94 apart,
        // well inside a tolerance of 2, and three-vertex spans span 2.8.
        let ring: Vec<DVec2> = (0..200)
            .map(|i| {
                let a = i as f64 / 200.0 * std::f64::consts::TAU;
                DVec2::new(30.0 * a.cos(), 30.0 * a.sin())
            })
            .collect();
        let mut despiked = ring.clone();
        despike(&mut despiked, 2.0);
        assert_eq!(despiked, ring, "a smooth curve was simplified");
    }

    /// A narrow feature that is still wider than the pen stays.
    #[test]
    fn a_real_notch_is_not_mistaken_for_a_spike() {
        let mut ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(40.0, 0.0),
            DVec2::new(40.0, 30.0),
            // A 6-wide slot cut into the top — narrow, but a cutter can make it.
            DVec2::new(23.0, 30.0),
            DVec2::new(23.0, 10.0),
            DVec2::new(17.0, 10.0),
            DVec2::new(17.0, 30.0),
            DVec2::new(0.0, 30.0),
        ];
        let before = ring.clone();
        despike(&mut ring, 1.26);
        assert_eq!(ring, before, "a 6-wide slot is not a hairline");
    }

    /// A hole in the fewest strips the app allows. Plan §8.8.
    ///
    /// Three strips used to unwrap wrong, leaving one panel edge-on to the page.
    /// Fitting a rim there divided by a determinant of `8e-14` and asked for a
    /// hole `1e14` wide, which `pieces` then walked one strip at a time — the app
    /// stopped responding the moment Divisions was set to 3.
    #[test]
    fn a_hole_in_a_three_strip_pattern_is_still_a_hole() {
        for phi_divisions in [3, 4, 5] {
            let input = EllipsoidInput {
                h_middle: 2.625,
                phi_divisions,
                ..EllipsoidInput::default()
            };
            let (p, g, f) = setup(&input);
            let diameter = 0.7;
            // Up in a petal, where the strip is tapering: the shape that first
            // showed this.
            let cutout = Cutout::hole(0.55, 0.72, diameter);
            let pieces = pieces(&p, &g, &f, &cutout);

            assert_eq!(pieces.len(), 1, "{phi_divisions} strips: hole was split");
            let span = extent(&pieces[0]);
            assert!(
                (span - diameter).abs() < diameter * 0.05,
                "{phi_divisions} strips: hole spans {span}, asked for {diameter}"
            );
        }
    }

    /// A rim that could not be fitted is dropped, not walked.
    ///
    /// The bound in `pieces` is the backstop for §8.8: whatever a future
    /// degeneracy does to a rim, the strip walk cannot become unbounded.
    #[test]
    fn an_impossibly_wide_shape_is_dropped_rather_than_walked() {
        let input = EllipsoidInput::default();
        let (p, g, f) = setup(&input);

        // Four turns wide — no hole, whatever produced it.
        let cutout = Cutout::Polygon {
            points: vec![[-2.0, 0.4], [2.0, 0.4], [2.0, 0.6], [-2.0, 0.6]],
        };
        assert!(pieces(&p, &g, &f, &cutout).is_empty());

        // One turn is still entertained, so the bound cannot be mistaken for a
        // clamp on ordinary seam-crossing shapes.
        let across_the_seam = Cutout::Polygon {
            points: vec![[-0.02, 0.4], [0.02, 0.4], [0.02, 0.6], [-0.02, 0.6]],
        };
        assert!(!pieces(&p, &g, &f, &across_the_seam).is_empty());
    }
}

#[cfg(test)]
mod seam_tests {
    use super::*;
    use crate::layout::PatternTransform;
    use ellipsoid_core::{EllipsoidInput, compute_flat_geometry, compute_geometry};

    /// Count the closed rings `cutouts` leave in the pattern.
    ///
    /// A cutout wholly inside the pattern is one hole, however many panel
    /// strips it happens to span — the pieces are adjacent, so the boolean
    /// should weld them. An extra ring means a chord was left along a seam.
    ///
    /// Takes the whole set rather than one shape because `draw_cutouts`
    /// subtracts them in a single pass, and a shape that welds on its own has
    /// been seen not to when other cutouts are in the same pass.
    fn rings(input: &EllipsoidInput, cutouts: &[Cutout]) -> usize {
        let geometry = compute_geometry(input);
        let flat = compute_flat_geometry(&geometry, input);
        let param = SurfaceParam::new(&geometry);
        let transform = PatternTransform::new(input, &flat);

        let pieces: Vec<Vec<DVec2>> = cutouts
            .iter()
            .flat_map(|c| pieces(&param, &geometry, &flat, c))
            .map(|piece| {
                piece
                    .into_iter()
                    .map(|p| transform.place_outline(p))
                    .collect()
            })
            .collect();

        // The cut outline, as `draw_cutouts` gets it.
        let scene = crate::layout::draw_edges(input, &flat);
        let outline = match scene
            .layer(crate::layout::LAYER_PATTERN)
            .and_then(|l| l.items.first())
        {
            Some(crate::Item::Path { points, .. }) => points.clone(),
            _ => panic!("no cut outline"),
        };

        subtract_from_outline(&outline, &pieces).1.len()
    }

    fn base() -> EllipsoidInput {
        EllipsoidInput {
            h_middle: 2.625,
            h_top_shift: 0.125,
            ..Default::default()
        }
    }

    #[test]
    fn a_shape_spanning_a_seam_is_still_one_hole() {
        let input = base();
        // Every seam, so a defect that only shows on some of them cannot hide.
        for strip in 1..input.phi_divisions {
            let seam = strip as f64 / input.phi_divisions as f64;
            let shape = Cutout::polygon(vec![
                [seam - 0.055, 0.38],
                [seam + 0.050, 0.38],
                [seam + 0.055, 0.46],
                [seam + 0.005, 0.53],
                [seam - 0.050, 0.46],
            ]);
            assert_eq!(
                rings(&input, &[shape]),
                1,
                "shape across the seam at u={seam} came back as more than one ring"
            );
        }
    }

    #[test]
    fn a_hole_spanning_a_seam_is_still_one_hole() {
        let input = base();
        for strip in 1..input.phi_divisions {
            let seam = strip as f64 / input.phi_divisions as f64;
            assert_eq!(
                rings(&input, &[Cutout::hole(seam, 0.45, 0.9)]),
                1,
                "hole across the seam at u={seam} came back as more than one ring"
            );
        }
    }
    /// Everything inside a drawn shape is removed, seams and all.
    ///
    /// The shape reported as leaving material behind: it crosses a seam, spans
    /// the joined band and both petal regions either side of it, and reaches
    /// the bottom edge. The clipped side along the seam used to be carried as a
    /// single chord, so a 0.6 in strip of panel survived down the middle of it.
    #[test]
    fn a_shape_drawn_across_a_seam_leaves_nothing_inside_it() {
        let input = EllipsoidInput {
            h_middle: 2.0,
            ..Default::default()
        };
        let shape = Cutout::polygon(vec![
            [0.407, 0.548],
            [0.442, 0.620],
            [0.463, 0.654],
            [0.533, 0.769],
            [0.537, 0.603],
            [0.564, 0.506],
            [0.537, 0.263],
            [0.535, 0.002],
            [0.409, 0.002],
        ]);

        let geometry = compute_geometry(&input);
        let flat = compute_flat_geometry(&geometry, &input);
        let param = SurfaceParam::new(&geometry);
        let transform = PatternTransform::new(&input, &flat);
        let placed: Vec<Vec<DVec2>> = pieces(&param, &geometry, &flat, &shape)
            .into_iter()
            .map(|p| p.into_iter().map(|q| transform.place_outline(q)).collect())
            .collect();

        let scene = crate::layout::draw_edges(&input, &flat);
        let outline = match scene
            .layer(crate::layout::LAYER_PATTERN)
            .and_then(|l| l.items.first())
        {
            Some(crate::Item::Path { points, .. }) => points.clone(),
            _ => panic!("no cut outline"),
        };

        let (outer, holes) = subtract_from_outline(&outline, &placed);
        assert!(
            holes.is_empty(),
            "the shape reaches the edge, so it notches"
        );

        // Cutting all the way through leaves the pattern in two pieces. A third
        // is the leftover strip; the two tiny triangles where the panels part
        // are dropped later, by the area filter in `draw_cutouts`.
        let substantial = outer
            .iter()
            .filter(|r| area(r) > transform.stroke_width().powi(2))
            .count();
        assert_eq!(substantial, 2, "expected the pattern to be cut in two");

        // And what went is what was asked for, to within the sliver of shape
        // that lies below the bottom edge and so was never there to remove.
        let removed = area(&outline) - outer.iter().map(|r| area(r)).sum::<f64>();
        let asked = placed.iter().map(|p| area(p)).sum::<f64>();
        assert!(
            (removed - asked).abs() < asked * 0.02,
            "removed {removed:.0} of {asked:.0} asked"
        );
    }

    /// Guide lines are fold marks, so they stop at a hole rather than crossing
    /// it — there is no material there to fold.
    #[test]
    fn guide_lines_do_not_cross_a_cutout() {
        let a = DVec2::new(0.0, 5.0);
        let b = DVec2::new(10.0, 5.0);
        let hole = vec![
            DVec2::new(4.0, 4.0),
            DVec2::new(6.0, 4.0),
            DVec2::new(6.0, 6.0),
            DVec2::new(4.0, 6.0),
        ];

        let parts = outside_pieces(a, b, std::slice::from_ref(&hole));
        assert_eq!(parts.len(), 2, "the segment should be cut in two");
        assert!((parts[0][1].x - 4.0).abs() < 1e-9, "{:?}", parts[0]);
        assert!((parts[1][0].x - 6.0).abs() < 1e-9, "{:?}", parts[1]);

        // Swallowed whole, so nothing is left of it.
        let inside = DVec2::new(4.5, 5.0);
        let also_inside = DVec2::new(5.5, 5.0);
        assert!(outside_pieces(inside, also_inside, std::slice::from_ref(&hole)).is_empty());

        // Clear of it, so it is untouched.
        let clear = outside_pieces(DVec2::new(0.0, 1.0), DVec2::new(10.0, 1.0), &[hole]);
        assert_eq!(clear.len(), 1);
    }
}
