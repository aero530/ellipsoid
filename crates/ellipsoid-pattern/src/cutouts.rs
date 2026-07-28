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

    let mut out = Vec::new();
    for strip in first..=last {
        let lo = strip as f64 / strips;
        let hi = (strip + 1) as f64 / strips;
        let clipped = clip_to_rect(&outline, DVec2::new(lo, 0.0), DVec2::new(hi, 1.0));

        // Fewer than three points is a tangential graze, not a piece.
        if clipped.len() < 3 {
            continue;
        }
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

    /// A shape spanning a seam the outline has *merged* should be one hole.
    ///
    /// **Known failure — see `RUST_CONVERSION_PLAN.md` Appendix O.** `min_gap`
    /// decides whether two panels are drawn as one piece. Where they are, there
    /// is no cut edge between them, so a shape crossing that seam has nothing
    /// to be divided by; splitting it anyway leaves a sliver of material a hair
    /// wide across the middle of the hole. [`pieces`] always divides at the
    /// strip boundary and knows nothing of `min_gap`, so it splits regardless.
    ///
    /// The second half of the test is what makes this hard to fix casually:
    /// where the panels really are further apart than `min_gap`, the seam *is*
    /// a cut edge and two holes is the right answer. Widening the pieces enough
    /// to fuse the first case makes them reach the panel edge in the second,
    /// turning two correct holes into notches.
    #[test]
    #[ignore = "known defect: pieces are split at every seam, merged or not"]
    fn merging_follows_min_gap() {
        let shape = Cutout::polygon(vec![
            [0.68, 0.36],
            [0.82, 0.36],
            [0.86, 0.50],
            [0.75, 0.58],
            [0.64, 0.50],
        ]);

        let merged = EllipsoidInput {
            min_gap: 0.005,
            ..base()
        };
        assert_eq!(
            rings(&merged, std::slice::from_ref(&shape)),
            1,
            "the panels are drawn as one piece, so the hole must be one ring"
        );

        // The default. The panels are 0.0013 in apart at these rows — wider
        // than `min_gap`, so the outline draws the seam and the shape is
        // genuinely divided between two panels.
        let separate = base();
        assert_eq!(separate.min_gap, 0.001);
        assert_eq!(
            rings(&separate, std::slice::from_ref(&shape)),
            2,
            "the panels are cut apart, so each must carry its own hole"
        );
    }
}
