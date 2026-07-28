//! Geometric primitives — port of `app/utils/geometryHelpers.js`.
//!
//! Every function here mirrors the original's order of operations rather than
//! using the equivalent `glam` convenience method. That is deliberate: these
//! feed the golden-file parity harness, and reassociating floating-point
//! arithmetic would introduce differences that are hard to distinguish from
//! genuine porting mistakes. `glam` is used for storage, not for the math.

use glam::DVec3;

/// Euclidean distance between two points.
pub fn distance(p0: DVec3, p1: DVec3) -> f64 {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let dz = p1.z - p0.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Normal of the plane through three points. Not normalised.
pub fn plane_normal(a: DVec3, b: DVec3, c: DVec3) -> DVec3 {
    let v1 = DVec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let v2 = DVec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    DVec3::new(
        v1.y * v2.z - v1.z * v2.y,
        v1.z * v2.x - v1.x * v2.z,
        v1.x * v2.y - v1.y * v2.x,
    )
}

/// Angle between plane `ABC` and plane `ABD`, which share the edge `AB`.
///
/// The absolute value on the dot product means this always returns the acute
/// angle. Callers in [`crate::flatten`] negate the result where the obtuse
/// angle is wanted — that is where several of the original's special cases live.
///
/// Returns `NaN` if the cosine lands outside `[-1, 1]` through rounding, or if
/// either normal is degenerate. The original had the same behaviour and no
/// golden case triggers it, so no clamp is applied here: silently clamping
/// would mask a real divergence.
pub fn angle_between_planes(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> f64 {
    let n1 = plane_normal(a, b, c);
    let n2 = plane_normal(a, b, d);

    ((n1.x * n2.x + n1.y * n2.y + n1.z * n2.z).abs()
        / ((n1.x * n1.x + n1.y * n1.y + n1.z * n1.z).sqrt()
            * (n2.x * n2.x + n2.y * n2.y + n2.z * n2.z).sqrt()))
    .acos()
}

/// The angle that swings the half-plane `ABC` about `AB` until it continues the
/// half-plane `ABD` — the angle that flattens the fold along `AB`.
///
/// [`angle_between_planes`] answers a different question. It reports the acute
/// angle between two *planes*, which happens to equal this whenever the two
/// *half-planes* meet obtusely: the unfold is `π − θ`, and for `θ > π/2` the
/// acute angle between the planes is `π − θ` as well. When the halves meet
/// acutely the two part company — the acute angle is `θ` itself, and unfolding
/// by it leaves the fold half-closed.
///
/// The original never made the distinction, and it does not matter for most of
/// the unrolling, where consecutive rows of a smooth surface meet at very nearly
/// a straight angle. It matters when unwrapping a cylinder of **three or four
/// strips**, where the strips genuinely meet at an acute angle. See
/// [`crate::flatten`] and the plan's §8.8.
pub fn unfold_angle(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> f64 {
    let angle = angle_between_planes(a, b, c, d);
    if half_planes_meet_acutely(a, b, c, d) {
        std::f64::consts::PI - angle
    } else {
        angle
    }
}

/// Whether the half-planes `ABC` and `ABD` meet at less than a right angle.
///
/// Measured on the components of `C` and `D` perpendicular to `AB`, which is
/// what distinguishes the two half-planes; the planes themselves cannot tell.
fn half_planes_meet_acutely(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> bool {
    let axis = (b - a).normalize_or_zero();
    let across = |p: DVec3| {
        let offset = p - a;
        offset - axis * offset.dot(axis)
    };
    across(c).dot(across(d)) > 0.0
}

/// Rotate `p0` about the axis through `p1` and `p2` by `theta` radians.
///
/// Rodrigues' rotation, built as an explicit matrix exactly as the original
/// did. Positive angles are counter-clockwise looking down the axis toward the
/// origin, in a right-handed system; the order of `p1`/`p2` together with the
/// sign of `theta` sets the direction.
///
/// Adapted from <http://paulbourke.net/geometry/rotate/>.
pub fn rotate_point(p1: DVec3, p2: DVec3, p0: DVec3, theta: f64) -> DVec3 {
    // Translate so the axis passes through the origin.
    let p = DVec3::new(p0.x - p1.x, p0.y - p1.y, p0.z - p1.z);

    let n = DVec3::new(p2.x - p1.x, p2.y - p1.y, p2.z - p1.z);
    let nm = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();

    // Axis unit vector.
    let x = n.x / nm;
    let y = n.y / nm;
    let z = n.z / nm;

    let c = theta.cos();
    let t = 1.0 - theta.cos();
    let s = theta.sin();

    let d11 = t * x * x + c;
    let d12 = t * x * y - s * z;
    let d13 = t * x * z + s * y;
    let d21 = t * x * y + s * z;
    let d22 = t * y * y + c;
    let d23 = t * y * z - s * x;
    let d31 = t * x * z - s * y;
    let d32 = t * y * z + s * x;
    let d33 = t * z * z + c;

    let q = DVec3::new(
        d11 * p.x + d12 * p.y + d13 * p.z,
        d21 * p.x + d22 * p.y + d23 * p.z,
        d31 * p.x + d32 * p.y + d33 * p.z,
    );

    // Translate back.
    DVec3::new(q.x + p1.x, q.y + p1.y, q.z + p1.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn close(a: DVec3, b: DVec3, tol: f64) -> bool {
        (a.x - b.x).abs() < tol && (a.y - b.y).abs() < tol && (a.z - b.z).abs() < tol
    }

    #[test]
    fn distance_is_euclidean() {
        assert_eq!(
            distance(DVec3::ZERO, DVec3::new(3.0, 4.0, 0.0)),
            5.0,
            "3-4-5 triangle"
        );
        assert_eq!(distance(DVec3::ONE, DVec3::ONE), 0.0);
    }

    #[test]
    fn plane_normal_is_the_cross_product_of_the_edges() {
        // XY plane -> normal along +z.
        let n = plane_normal(
            DVec3::ZERO,
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(n, DVec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn quarter_turn_about_z() {
        // Axis = z through the origin; +x should land on +y.
        let r = rotate_point(
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::new(1.0, 0.0, 0.0),
            PI / 2.0,
        );
        assert!(close(r, DVec3::new(0.0, 1.0, 0.0), 1e-15), "got {r:?}");
    }

    #[test]
    fn rotation_direction_follows_axis_order() {
        // Reversing p1/p2 reverses the sense of rotation.
        let fwd = rotate_point(DVec3::ZERO, DVec3::Z, DVec3::new(1.0, 0.0, 0.0), PI / 2.0);
        let rev = rotate_point(DVec3::ZERO, -DVec3::Z, DVec3::new(1.0, 0.0, 0.0), PI / 2.0);
        assert!(close(fwd, DVec3::new(0.0, 1.0, 0.0), 1e-15));
        assert!(close(rev, DVec3::new(0.0, -1.0, 0.0), 1e-15));
    }

    #[test]
    fn points_on_the_axis_are_invariant() {
        // The cylinder-unwrap step in `flatten` rotates its own axis endpoints,
        // relying on this being a no-op.
        let p1 = DVec3::new(1.0, 2.0, 3.0);
        let p2 = DVec3::new(4.0, 6.0, 9.0);
        for t in [0.3, 1.0, -2.2] {
            assert!(close(rotate_point(p1, p2, p1, t), p1, 1e-12));
            assert!(close(rotate_point(p1, p2, p2, t), p2, 1e-12));
        }
    }

    #[test]
    fn rotation_preserves_distance_to_the_axis() {
        let p1 = DVec3::new(-1.0, 0.5, 2.0);
        let p2 = DVec3::new(2.0, -3.0, 0.25);
        let p0 = DVec3::new(5.0, 1.0, -2.0);
        let before = distance(p1, p0);
        for t in [0.1, 1.7, -0.9, 3.0] {
            let after = distance(p1, rotate_point(p1, p2, p0, t));
            assert!((before - after).abs() < 1e-12, "theta={t}");
        }
    }

    #[test]
    fn agrees_with_glam_quaternion() {
        // Sanity check against an independent implementation. Loose tolerance:
        // the point is that the formula is right, not that the bits match.
        let p1 = DVec3::new(0.5, -1.0, 0.25);
        let p2 = DVec3::new(2.0, 1.5, -0.75);
        let p0 = DVec3::new(-1.0, 2.0, 3.0);
        let axis = (p2 - p1).normalize();

        for theta in [0.25, 1.1, -2.0] {
            let mine = rotate_point(p1, p2, p0, theta);
            let theirs = p1 + glam::DQuat::from_axis_angle(axis, theta) * (p0 - p1);
            assert!(close(mine, theirs, 1e-12), "theta={theta}");
        }
    }

    #[test]
    fn angle_between_planes_is_acute_and_symmetric() {
        let a = DVec3::ZERO;
        let b = DVec3::new(1.0, 0.0, 0.0);
        let c = DVec3::new(0.0, 1.0, 0.0); // XY plane
        let d = DVec3::new(0.0, 0.0, 1.0); // XZ plane

        let angle = angle_between_planes(a, b, c, d);
        assert!((angle - PI / 2.0).abs() < 1e-15, "perpendicular planes");

        // The abs() on the dot product forces the acute angle, so swapping the
        // two off-edge points cannot change the result.
        assert_eq!(angle, angle_between_planes(a, b, d, c));
    }

    /// The angle that flattens a fold, whichever way the fold leans. Plan §8.8.
    #[test]
    fn unfold_angle_flattens_the_fold() {
        let a = DVec3::ZERO;
        let b = DVec3::new(1.0, 0.0, 0.0);
        // The reference half-plane, pointing along +y.
        let d = DVec3::new(0.5, 1.0, 0.0);

        for degrees in [10.0, 45.0, 89.0, 90.0, 91.0, 135.0, 170.0] {
            let fold = degrees * PI / 180.0;
            // A half-plane `fold` away from `d`, about the x axis.
            let c = DVec3::new(0.5, fold.cos(), fold.sin());

            let unfold = unfold_angle(a, b, c, d);
            assert!(
                (unfold - (PI - fold)).abs() < 1e-12,
                "{degrees}°: unfold {unfold} wanted {}",
                PI - fold
            );

            // Which is what `angle_between_planes` gives only for obtuse folds —
            // the whole point of the distinction.
            let acute = angle_between_planes(a, b, c, d);
            if degrees > 90.0 {
                assert!((acute - unfold).abs() < 1e-12, "{degrees}°");
            } else if degrees < 90.0 {
                assert!(acute < unfold, "{degrees}°: {acute} vs {unfold}");
            }
        }
    }

    #[test]
    fn coplanar_points_give_zero_angle() {
        let angle = angle_between_planes(
            DVec3::ZERO,
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, 2.0, 0.0),
        );
        assert!(angle.abs() < 1e-8, "got {angle}");
    }
}
