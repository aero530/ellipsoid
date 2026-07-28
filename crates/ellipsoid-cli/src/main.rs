//! Headless ellipsoid flat-pattern generator.
//!
//! # Status
//!
//! All output formats work. Phase 3 adds the remaining surface: `--cutouts`
//! once Phase 6 lands, and snapshot coverage.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use ellipsoid_core::{
    EllipsoidInput, Projection, Unit, compute_flat_geometry, compute_geometry, flat_to_obj,
    geometry_to_obj,
};
use ellipsoid_pattern::{SvgOptions, build_scene, to_svg};

/// Generate a flat pattern for an arbitrary ellipsoid.
///
/// Unspecified parameters come from --config, or from built-in defaults when
/// there is no config file. Run with `--format config` to print the resolved
/// values, which can be saved and fed back in with --config.
#[derive(Debug, Parser)]
#[command(
    name = "ellipsoid",
    version,
    about,
    // Angles are legitimately negative, and letting a negative slip through to
    // validation gives "a: must be greater than 0" rather than clap's
    // "unexpected argument '-1'".
    allow_negative_numbers = true,
    after_long_help = "\
EXAMPLES:
  # Flat pattern for a helmet blank, to a file
  ellipsoid --a 3.75 --b 2.875 --c 3 --h-middle 2 -o helmet.svg

  # Spherical projection in millimetres, straight to stdout
  ellipsoid --projection spherical --units mm

  # Save the resolved settings, then reuse and tweak them
  ellipsoid --format config -o helmet.json
  ellipsoid --config helmet.json --divisions-phi 12 -o helmet.svg

  # 3D meshes of the ellipsoid and of the flattened pattern
  ellipsoid --format obj      -o helmet.obj
  ellipsoid --format obj-flat -o helmet-flat.obj"
)]
struct Cli {
    /// Load parameters from a JSON settings file. Individual flags override it.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Output file. Use `-` for stdout.
    #[arg(short, long, value_name = "FILE", default_value = "-")]
    output: PathBuf,

    /// What to emit.
    #[arg(long, value_enum, default_value_t = Format::Svg)]
    format: Format,

    // --- Geometry -----------------------------------------------------------
    /// Semi axis a.
    #[arg(long, value_name = "LEN", help_heading = "Geometry")]
    a: Option<f64>,
    /// Semi axis b.
    #[arg(long, value_name = "LEN", help_heading = "Geometry")]
    b: Option<f64>,
    /// Semi axis c.
    #[arg(long, value_name = "LEN", help_heading = "Geometry")]
    c: Option<f64>,

    /// Added height at top of open ellipsoid (theta max < 90).
    #[arg(long, value_name = "LEN", help_heading = "Geometry")]
    h_top: Option<f64>,
    /// Added thickness in the middle of the ellipsoid (vertically).
    #[arg(long, value_name = "LEN", help_heading = "Geometry")]
    h_middle: Option<f64>,
    /// Added height at the bottom of an open ellipsoid (theta min > -90).
    #[arg(long, value_name = "LEN", help_heading = "Geometry")]
    h_bottom: Option<f64>,
    /// Scaling factor put on the hTop ellipse (based on the ellipse at thetaMax).
    #[arg(long, value_name = "F", help_heading = "Geometry")]
    h_top_fraction: Option<f64>,
    /// Factor used to shift the hTop ellipse side to side.
    #[arg(long, value_name = "F", help_heading = "Geometry")]
    h_top_shift: Option<f64>,

    /// Angle defining the bottom of the ellipsoid; -90 is fully closed.
    #[arg(long, value_name = "DEG", help_heading = "Geometry")]
    theta_min: Option<f64>,
    /// Angle defining the top of the ellipsoid; 90 is fully closed.
    #[arg(long, value_name = "DEG", help_heading = "Geometry")]
    theta_max: Option<f64>,

    /// Number of longitudinal divisions of the ellipsoid (around).
    #[arg(long, value_name = "N", help_heading = "Geometry")]
    divisions_phi: Option<usize>,
    /// Number of latitudinal divisions of the ellipsoid (pole to pole).
    #[arg(long, value_name = "N", help_heading = "Geometry")]
    divisions_theta: Option<usize>,

    // --- Pattern ------------------------------------------------------------
    /// Spherical unfolds from the top; cylindrical unfolds from the front.
    #[arg(long, value_enum, help_heading = "Pattern")]
    projection: Option<ProjectionArg>,
    /// Output unit.
    #[arg(long, value_enum, help_heading = "Pattern")]
    units: Option<UnitArg>,
    /// Padding in the SVG around the ellipsoid pattern.
    #[arg(long, value_name = "LEN", help_heading = "Pattern")]
    image_offset: Option<f64>,
    /// Minimum gap between lines; allows for cutting tool radius.
    #[arg(long, value_name = "LEN", help_heading = "Pattern")]
    min_gap: Option<f64>,

    /// Save the SVG with Inkscape layers.
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        overrides_with = "no_inkscape_layers",
        help_heading = "Pattern"
    )]
    inkscape_layers: Option<bool>,
    /// Write a plain SVG with no Inkscape layer attributes.
    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        overrides_with = "inkscape_layers",
        help_heading = "Pattern"
    )]
    no_inkscape_layers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Flat pattern as SVG.
    Svg,
    /// Ellipsoid mesh as Wavefront OBJ.
    Obj,
    /// Flattened pattern mesh as Wavefront OBJ.
    ObjFlat,
    /// Resolved parameters as JSON — useful for building settings files.
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ProjectionArg {
    Spherical,
    Cylindrical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum UnitArg {
    In,
    Mm,
    Cm,
}

impl From<ProjectionArg> for Projection {
    fn from(v: ProjectionArg) -> Self {
        match v {
            ProjectionArg::Spherical => Projection::Spherical,
            ProjectionArg::Cylindrical => Projection::Cylindrical,
        }
    }
}

impl From<UnitArg> for Unit {
    fn from(v: UnitArg) -> Self {
        match v {
            UnitArg::In => Unit::Inch,
            UnitArg::Mm => Unit::Mm,
            UnitArg::Cm => Unit::Cm,
        }
    }
}

impl Cli {
    /// Start from defaults or `--config`, then apply each explicitly-passed flag.
    fn resolve_input(&self) -> Result<EllipsoidInput, String> {
        let mut input = match &self.config {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| format!("reading {}: {e}", path.display()))?;
                EllipsoidInput::from_json(&text)
                    .map_err(|e| format!("parsing {}: {e}", path.display()))?
            }
            None => EllipsoidInput::default(),
        };

        macro_rules! set {
            ($($field:ident),* $(,)?) => {
                $(if let Some(v) = self.$field { input.$field = v; })*
            };
        }
        set!(
            a,
            b,
            c,
            h_top,
            h_middle,
            h_bottom,
            h_top_fraction,
            h_top_shift,
            theta_min,
            theta_max,
            image_offset,
            min_gap,
            inkscape_layers,
        );

        // `overrides_with` makes the two flags mutually cancelling, so last one
        // on the command line wins.
        if self.no_inkscape_layers {
            input.inkscape_layers = false;
        }

        if let Some(v) = self.divisions_phi {
            input.phi_divisions = v;
        }
        if let Some(v) = self.divisions_theta {
            input.theta_divisions = v;
        }
        if let Some(v) = self.projection {
            input.projection = v.into();
        }
        if let Some(v) = self.units {
            input.unit = v.into();
        }

        Ok(input)
    }
}

fn write_output(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    if path.as_os_str() == "-" {
        std::io::stdout()
            .write_all(bytes)
            .map_err(|e| format!("writing stdout: {e}"))
    } else {
        std::fs::write(path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let input = cli.resolve_input()?;

    // Refuse clearly rather than emit a garbage pattern. `compute_geometry`
    // stays a faithful port and would silently coerce these.
    if let Err(problems) = input.validate() {
        let mut msg = String::from("invalid parameters:");
        for problem in &problems {
            msg.push_str(&format!("\n  {problem}"));
        }
        return Err(msg);
    }

    let bytes = match cli.format {
        Format::Config => {
            let mut s = serde_json::to_string_pretty(&input)
                .map_err(|e| format!("serializing config: {e}"))?;
            s.push('\n');
            s.into_bytes()
        }
        Format::Obj => {
            let geometry = compute_geometry(&input);
            geometry_to_obj(&geometry).into_bytes()
        }
        Format::ObjFlat => {
            let geometry = compute_geometry(&input);
            let flat = compute_flat_geometry(&geometry, &input);
            flat_to_obj(&flat).into_bytes()
        }
        Format::Svg => {
            let geometry = compute_geometry(&input);
            let flat = compute_flat_geometry(&geometry, &input);
            let scene = build_scene(&input, &geometry, &flat);
            to_svg(
                &scene,
                &SvgOptions {
                    inkscape_layers: input.inkscape_layers,
                },
            )
            .into_bytes()
        }
    };

    write_output(&cli.output, &bytes)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn flags_override_defaults() {
        let cli = Cli::parse_from(["ellipsoid", "--a", "5", "--theta-min", "-90"]);
        let input = cli.resolve_input().unwrap();
        assert_eq!(input.a, 5.0);
        assert_eq!(input.theta_min, -90.0);
        // Untouched fields keep their defaults.
        assert_eq!(input.b, EllipsoidInput::default().b);
    }

    #[test]
    fn negative_angles_parse_without_being_read_as_flags() {
        let cli = Cli::parse_from(["ellipsoid", "--theta-max", "-20"]);
        assert_eq!(cli.resolve_input().unwrap().theta_max, -20.0);
    }
}
