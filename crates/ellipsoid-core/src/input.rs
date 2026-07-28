//! The authoritative input state: every parameter a pattern is derived from.

use serde::{Deserialize, Serialize};

use crate::surface::Cutout;
use crate::units::Unit;

/// How the ellipsoid surface is unrolled into a flat pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Projection {
    /// Unfold from the top of the ellipsoid.
    Spherical,
    /// Unfold from the front of the ellipsoid.
    #[default]
    Cylindrical,
}

impl Projection {
    /// All projections, for populating pickers.
    pub const ALL: [Projection; 2] = [Projection::Spherical, Projection::Cylindrical];

    /// Lowercase name, matching the string the JavaScript used.
    pub const fn name(self) -> &'static str {
        match self {
            Projection::Spherical => "spherical",
            Projection::Cylindrical => "cylindrical",
        }
    }
}

impl std::fmt::Display for Projection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Every parameter that defines a pattern.
///
/// Field names follow the plan's renaming (`RUST_CONVERSION_PLAN.md` §3): the
/// original distinguished `Divisions` from `divisions` by capitalization alone.
///
/// Lengths are in [`Unit`]s, angles in degrees. Defaults mirror the initial
/// Redux state in `app/reducers/input.js`.
///
/// Not `Copy`: [`cutouts`](Self::cutouts) is a `Vec`. Clone it explicitly where
/// a scratch copy is wanted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EllipsoidInput {
    /// Semi-axis along x.
    pub a: f64,
    /// Semi-axis along y.
    pub b: f64,
    /// Semi-axis along z (height).
    pub c: f64,

    /// Height added above an open top (`theta_max < 90`).
    pub h_top: f64,
    /// Height inserted at the equator, splitting the ellipsoid vertically.
    pub h_middle: f64,
    /// Height added below an open bottom (`theta_min > -90`).
    pub h_bottom: f64,
    /// Scale applied to the added top ellipse, relative to the one at `theta_max`.
    pub h_top_fraction: f64,
    /// Sideways shift of the added top ellipse.
    pub h_top_shift: f64,

    /// Longitudinal divisions, around the circumference. Was `Divisions`.
    pub phi_divisions: usize,
    /// Latitudinal divisions, pole to pole. Was `divisions`.
    pub theta_divisions: usize,

    /// Angle defining the bottom of the ellipsoid; -90 is fully closed.
    pub theta_min: f64,
    /// Angle defining the top of the ellipsoid; 90 is fully closed.
    pub theta_max: f64,

    /// Output unit. Replaces the raw `ppu` float.
    pub unit: Unit,
    /// Padding around the pattern in the output image.
    pub image_offset: f64,
    /// Minimum gap between lines; points closer than this are merged.
    pub min_gap: f64,
    /// Unrolling strategy.
    pub projection: Projection,
    /// Emit Inkscape `groupmode="layer"` attributes on groups.
    pub inkscape_layers: bool,

    /// Holes to cut, in resolution-independent surface coordinates.
    ///
    /// Part of the input rather than separate state so they travel with a saved
    /// settings file and through the CLI's `--config` for free.
    ///
    /// Omitted from the serialised form when empty, so settings files written
    /// before cutouts existed stay byte-identical.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cutouts: Vec<Cutout>,
}

impl Default for EllipsoidInput {
    fn default() -> Self {
        Self {
            a: 3.75,
            b: 2.875,
            c: 3.0,
            h_top: 0.0,
            h_middle: 2.0,
            h_bottom: 2.0,
            h_top_fraction: 1.0,
            h_top_shift: 0.0,
            phi_divisions: 8,
            theta_divisions: 16,
            theta_min: -35.0,
            theta_max: 90.0,
            unit: Unit::Inch,
            image_offset: 0.5,
            min_gap: 0.001,
            projection: Projection::Cylindrical,
            inkscape_layers: true,
            cutouts: Vec::new(),
        }
    }
}

/// A parameter that would produce a broken or meaningless pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Field name as it appears in the serialised form.
    pub field: &'static str,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl EllipsoidInput {
    /// Parse settings JSON, the form [`Self::to_json`] writes.
    ///
    /// Tolerates a leading byte-order mark. Windows editors and PowerShell's
    /// `Set-Content -Encoding utf8` add one, and `serde_json` rejects it as a
    /// stray character — a confusing way for a hand-edited file to fail.
    ///
    /// Missing fields take their defaults and unknown fields are an error, so a
    /// partial file works and a typo does not silently do nothing.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(text))
    }

    /// Settings as indented JSON, for a file a person may want to edit.
    pub fn to_json(&self) -> String {
        // The derive cannot fail: every field is a number, string, bool or Vec
        // of those. Falling back to `{}` keeps the signature honest without
        // making every caller handle an impossible error.
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    /// Check for parameters that cannot produce a usable pattern.
    ///
    /// Deliberately *not* called by [`crate::compute_geometry`], which stays a
    /// faithful port and silently coerces bad values the way the original did
    /// (NaN becomes 0, divisions clamp up to 3). Front-ends call this first so
    /// they can refuse clearly instead of emitting a garbage pattern.
    ///
    /// Returns every problem found, not just the first.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        let mut check = |field: &'static str, value: f64, ok: bool, want: &str| {
            if !value.is_finite() {
                errors.push(ValidationError {
                    field,
                    message: format!("must be a finite number, got {value}"),
                });
            } else if !ok {
                errors.push(ValidationError {
                    field,
                    message: format!("must be {want}, got {value}"),
                });
            }
        };

        check("a", self.a, self.a > 0.0, "greater than 0");
        check("b", self.b, self.b > 0.0, "greater than 0");
        check("c", self.c, self.c > 0.0, "greater than 0");

        check("h_top", self.h_top, self.h_top >= 0.0, "0 or greater");
        check(
            "h_middle",
            self.h_middle,
            self.h_middle >= 0.0,
            "0 or greater",
        );
        check(
            "h_bottom",
            self.h_bottom,
            self.h_bottom >= 0.0,
            "0 or greater",
        );
        check(
            "h_top_fraction",
            self.h_top_fraction,
            self.h_top_fraction > 0.0,
            "greater than 0",
        );
        check("h_top_shift", self.h_top_shift, true, "");

        check(
            "theta_min",
            self.theta_min,
            (-90.0..=90.0).contains(&self.theta_min),
            "between -90 and 90",
        );
        check(
            "theta_max",
            self.theta_max,
            (-90.0..=90.0).contains(&self.theta_max),
            "between -90 and 90",
        );
        check(
            "image_offset",
            self.image_offset,
            self.image_offset >= 0.0,
            "0 or greater",
        );
        check("min_gap", self.min_gap, self.min_gap >= 0.0, "0 or greater");

        if self.theta_min.is_finite()
            && self.theta_max.is_finite()
            && self.theta_min >= self.theta_max
        {
            errors.push(ValidationError {
                field: "theta_min",
                message: format!(
                    "must be less than theta_max ({} >= {})",
                    self.theta_min, self.theta_max
                ),
            });
        }

        // compute_geometry clamps these up to 3 rather than failing. Front-ends
        // should say so instead of quietly producing a different shape.
        if self.phi_divisions < 3 {
            errors.push(ValidationError {
                field: "phi_divisions",
                message: format!("must be 3 or more, got {}", self.phi_divisions),
            });
        }
        if self.theta_divisions < 3 {
            errors.push(ValidationError {
                field: "theta_divisions",
                message: format!("must be 3 or more, got {}", self.theta_divisions),
            });
        }

        for (i, cutout) in self.cutouts.iter().enumerate() {
            if !cutout.is_valid() {
                errors.push(ValidationError {
                    field: "cutouts",
                    message: format!(
                        "cutout {i} ({}) has a non-finite value, a diameter of zero, \
                         or fewer than three points",
                        cutout.describe()
                    ),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// SVG user units per unit of length, for the selected [`Unit`].
    pub fn px_per_unit(&self) -> f64 {
        self.unit.px_per_unit()
    }

    /// Suggested output filename stem, e.g. `ellipsoid_a3.75in_b2.88in_c3.00in`.
    ///
    /// Matches the JavaScript, which formatted `a`/`b`/`c` with `toFixed(2)`.
    pub fn filename_stem(&self) -> String {
        let u = self.unit.suffix();
        format!(
            "ellipsoid_a{:.2}{u}_b{:.2}{u}_c{:.2}{u}",
            self.a, self.b, self.c
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy_redux_state() {
        let d = EllipsoidInput::default();
        assert_eq!(d.a, 3.75);
        assert_eq!(d.b, 2.875);
        assert_eq!(d.c, 3.0);
        assert_eq!(d.phi_divisions, 8);
        assert_eq!(d.theta_divisions, 16);
        assert_eq!(d.theta_min, -35.0);
        assert_eq!(d.theta_max, 90.0);
        assert_eq!(d.projection, Projection::Cylindrical);
        assert!(d.inkscape_layers);
    }

    #[test]
    fn filename_stem_matches_legacy_format() {
        // The JS built this from toFixed(2) of a, b, c.
        assert_eq!(
            EllipsoidInput::default().filename_stem(),
            "ellipsoid_a3.75in_b2.88in_c3.00in"
        );
    }

    #[test]
    fn defaults_are_valid() {
        assert_eq!(EllipsoidInput::default().validate(), Ok(()));
    }

    #[test]
    fn rejects_an_inverted_theta_range() {
        let input = EllipsoidInput {
            theta_min: 40.0,
            theta_max: 10.0,
            ..Default::default()
        };
        let errors = input.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.field == "theta_min"), "{errors:?}");
    }

    #[test]
    fn rejects_nonpositive_axes_and_nan() {
        let input = EllipsoidInput {
            a: 0.0,
            b: -1.0,
            c: f64::NAN,
            ..Default::default()
        };
        let errors = input.validate().unwrap_err();
        // Every problem is reported, not just the first.
        for field in ["a", "b", "c"] {
            assert!(
                errors.iter().any(|e| e.field == field),
                "{field}: {errors:?}"
            );
        }
    }

    #[test]
    fn rejects_division_counts_below_the_clamp() {
        // compute_geometry would silently raise these to 3; the user should be
        // told rather than handed a different shape.
        let input = EllipsoidInput {
            phi_divisions: 2,
            theta_divisions: 0,
            ..Default::default()
        };
        let errors = input.validate().unwrap_err();
        assert_eq!(errors.len(), 2, "{errors:?}");
    }

    #[test]
    fn accepts_the_extremes_of_the_theta_range() {
        let input = EllipsoidInput {
            theta_min: -90.0,
            theta_max: 90.0,
            ..Default::default()
        };
        assert_eq!(input.validate(), Ok(()));
    }

    #[test]
    fn round_trips_through_json() {
        let input = EllipsoidInput {
            a: 1.25,
            projection: Projection::Spherical,
            unit: Unit::Mm,
            cutouts: vec![
                Cutout::hole(0.25, 0.5, 0.125),
                Cutout::polygon(vec![[0.1, 0.2], [0.3, 0.2], [0.2, 0.4]]),
            ],
            ..Default::default()
        };
        assert_eq!(EllipsoidInput::from_json(&input.to_json()).unwrap(), input);
    }

    #[test]
    fn a_byte_order_mark_does_not_stop_a_file_loading() {
        // Windows editors and PowerShell's `Set-Content -Encoding utf8` add
        // one, and it is invisible in the file that fails to load.
        let json = format!("\u{feff}{}", EllipsoidInput::default().to_json());
        assert_eq!(
            EllipsoidInput::from_json(&json).unwrap(),
            EllipsoidInput::default()
        );
    }

    #[test]
    fn a_partial_file_fills_the_rest_from_the_defaults() {
        let input = EllipsoidInput::from_json(r#"{"a": 9.0}"#).expect("partial settings");
        assert_eq!(input.a, 9.0);
        assert_eq!(input.b, EllipsoidInput::default().b);
    }

    #[test]
    fn a_misspelled_field_is_an_error_rather_than_a_silent_no_op() {
        // `deny_unknown_fields`, so a hand-edited file says what is wrong
        // instead of loading and quietly ignoring the setting.
        let why = EllipsoidInput::from_json(r#"{"aa": 9.0}"#)
            .expect_err("unknown field should be rejected")
            .to_string();
        assert!(why.contains("aa"), "unhelpful message: {why}");
    }
}
