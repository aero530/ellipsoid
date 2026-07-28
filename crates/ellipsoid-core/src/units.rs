//! Drawing units.
//!
//! The JavaScript original stored a bare `ppu` float and recovered the unit by
//! comparing it against `96`, `3.7795276`, and `37.795276` with `===` — including
//! against the *string* forms — returning `null` on a miss. See `getUnits` in
//! `app/utils/ellipsoid.js`. An enum makes that failure mode unrepresentable.

use serde::{Deserialize, Serialize};

/// Unit of length for pattern output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    /// Inches. 96 px/in is the Inkscape and CSS standard.
    #[default]
    Inch,
    /// Millimeters.
    Mm,
    /// Centimeters.
    Cm,
}

impl Unit {
    /// SVG user units (pixels) per unit of length.
    ///
    /// These are the exact constants the JavaScript used, preserved so pattern
    /// output stays dimensionally identical.
    pub const fn px_per_unit(self) -> f64 {
        match self {
            Unit::Inch => 96.0,
            Unit::Mm => 3.779_527_6,
            Unit::Cm => 37.795_276,
        }
    }

    /// Short suffix used in generated filenames and ruler labels.
    pub const fn suffix(self) -> &'static str {
        match self {
            Unit::Inch => "in",
            Unit::Mm => "mm",
            Unit::Cm => "cm",
        }
    }

    /// All units, for populating pickers.
    pub const ALL: [Unit; 3] = [Unit::Inch, Unit::Mm, Unit::Cm];
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.suffix())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_legacy_ppu_constants() {
        assert_eq!(Unit::Inch.px_per_unit(), 96.0);
        assert_eq!(Unit::Mm.px_per_unit(), 3.7795276);
        assert_eq!(Unit::Cm.px_per_unit(), 37.795276);
    }
}
