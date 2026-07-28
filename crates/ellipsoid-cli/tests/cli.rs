//! End-to-end tests for the `ellipsoid` binary.
//!
//! Geometry correctness is covered by the golden-file harnesses in
//! `ellipsoid-core` and `ellipsoid-pattern`. These tests are about the CLI
//! itself: argument mapping, config merging, output routing, validation, and
//! that the binary produces exactly what the library does.
//!
//! Snapshots live in `tests/snapshots/`. Regenerate with:
//!
//! ```text
//! UPDATE_SNAPSHOTS=1 cargo test -p ellipsoid-cli
//! ```
//!
//! The SVG is compared byte for byte — it is written at fixed precision, so it
//! is stable everywhere. OBJ is compared as parsed numbers; see
//! [`assert_obj_snapshot`].

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ellipsoid_core::obj::parse_obj;
use ellipsoid_core::{EllipsoidInput, compute_flat_geometry, compute_geometry, geometry_to_obj};
use ellipsoid_pattern::{SvgOptions, build_scene, to_svg};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ellipsoid")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("running {:?}: {e}", args))
}

fn stdout_of(args: &[&str]) -> String {
    let out = run(args);
    assert!(
        out.status.success(),
        "{args:?} failed ({}):\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// `.gitattributes` sets `* text eol=lf`, so checked-out snapshots may arrive
/// with either ending depending on the platform. Compare on content.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
}

/// The stored snapshot, or `None` when regenerating — in which case `actual`
/// has just been written in its place and there is nothing to compare against.
fn stored_snapshot(name: &str, actual: &str) -> Option<String> {
    let path = snapshot_dir().join(name);

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(snapshot_dir()).expect("create snapshot dir");
        std::fs::write(&path, actual).unwrap_or_else(|e| panic!("writing {path:?}: {e}"));
        return None;
    }

    Some(std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing snapshot {path:?} ({e}).\n\
             Create it with: UPDATE_SNAPSHOTS=1 cargo test -p ellipsoid-cli"
        )
    }))
}

fn assert_snapshot(name: &str, actual: &str) {
    let Some(expected) = stored_snapshot(name, actual) else {
        return;
    };

    let (expected, actual) = (normalize(&expected), normalize(actual));
    if expected != actual {
        // Point at the first differing line rather than dumping both files.
        let at = expected
            .lines()
            .zip(actual.lines())
            .position(|(a, b)| a != b);
        let detail = match at {
            Some(i) => format!(
                "line {}:\n  expected: {}\n  actual:   {}",
                i + 1,
                expected.lines().nth(i).unwrap_or(""),
                actual.lines().nth(i).unwrap_or("")
            ),
            None => format!(
                "length differs: expected {} lines, got {}",
                expected.lines().count(),
                actual.lines().count()
            ),
        };
        panic!(
            "snapshot {name} does not match; {detail}\n\
             If this change is intended: UPDATE_SNAPSHOTS=1 cargo test -p ellipsoid-cli"
        );
    }
}

/// A snapshot is this implementation's own output, so it should agree to the
/// bit — but OBJ vertices are written at full `f64` precision, and the last
/// place or two of a flattened one depends on the platform's libm. One set of
/// snapshots is checked against both Linux and Windows in CI, so a byte
/// comparison cannot pass on both at once. Same reasoning, and same tolerance,
/// as the golden harness in `ellipsoid-core/tests/snapshots.rs`.
const OBJ_TOL: f64 = 1e-12;

fn close(a: f64, b: f64) -> bool {
    let scale = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() <= OBJ_TOL * scale
}

/// Compare an OBJ snapshot as parsed numbers: face topology and vertex count
/// exactly, coordinates to [`OBJ_TOL`].
///
/// This is what `ellipsoid-core`'s golden files do, and what the note in
/// `obj.rs` means by "tests compare parsed numbers, never text".
fn assert_obj_snapshot(name: &str, actual: &str) {
    let Some(expected) = stored_snapshot(name, actual) else {
        return;
    };

    let (want_vertices, want_faces) = parse_obj(&expected);
    let (got_vertices, got_faces) = parse_obj(actual);

    // Point at the first difference rather than dumping both meshes.
    let detail = if got_faces != want_faces {
        Some("face topology differs".to_string())
    } else if got_vertices.len() != want_vertices.len() {
        Some(format!(
            "vertex count: expected {}, got {}",
            want_vertices.len(),
            got_vertices.len()
        ))
    } else {
        got_vertices
            .iter()
            .zip(&want_vertices)
            .enumerate()
            .find_map(|(i, (got, want))| {
                (0..3)
                    .find(|&axis| !close(got[axis], want[axis]))
                    .map(|axis| {
                        format!(
                            "vertex {} axis {axis}:\n  expected: {}\n  actual:   {}",
                            i + 1,
                            want[axis],
                            got[axis]
                        )
                    })
            })
    };

    if let Some(detail) = detail {
        panic!(
            "snapshot {name} does not match; {detail}\n\
             If this change is intended: UPDATE_SNAPSHOTS=1 cargo test -p ellipsoid-cli"
        );
    }
}

/// A small, fully-specified pattern — small enough that the snapshot stays
/// reviewable, but with notes enabled so the ruler and labels are covered.
const SMALL: &[&str] = &[
    "--a",
    "2",
    "--b",
    "1.5",
    "--c",
    "1.75",
    "--divisions-phi",
    "4",
    "--divisions-theta",
    "4",
    "--h-middle",
    "0.5",
    "--h-bottom",
    "0.25",
];

// ---------------------------------------------------------------------------
// Output routing
// ---------------------------------------------------------------------------

#[test]
fn writes_to_stdout_by_default() {
    let svg = stdout_of(&["--divisions-phi", "3", "--divisions-theta", "3"]);
    assert!(
        svg.starts_with("<?xml"),
        "got: {}",
        &svg[..40.min(svg.len())]
    );
    assert!(svg.trim_end().ends_with("</svg>"));
}

#[test]
fn writes_to_a_file_when_asked() {
    let dir = tempdir();
    let path = dir.join("out.svg");
    let out = run(&["-o", path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty(),
        "file mode must not also write stdout"
    );

    let svg = std::fs::read_to_string(&path).expect("output file");
    assert!(svg.starts_with("<?xml"));
}

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

#[test]
fn svg_snapshot() {
    assert_snapshot("small.svg", &stdout_of(SMALL));
}

#[test]
fn obj_snapshot() {
    let mut args = SMALL.to_vec();
    args.extend_from_slice(&["--format", "obj"]);
    assert_obj_snapshot("small.obj", &stdout_of(&args));
}

#[test]
fn obj_flat_snapshot() {
    let mut args = SMALL.to_vec();
    args.extend_from_slice(&["--format", "obj-flat"]);
    assert_obj_snapshot("small-flat.obj", &stdout_of(&args));
}

// ---------------------------------------------------------------------------
// The binary must agree with the library
// ---------------------------------------------------------------------------

#[test]
fn binary_output_matches_the_library() {
    // Guards the arg -> EllipsoidInput mapping. If a flag were wired to the
    // wrong field, geometry parity would still pass but this would not.
    let input = EllipsoidInput {
        a: 2.0,
        b: 1.5,
        c: 1.75,
        phi_divisions: 4,
        theta_divisions: 4,
        h_middle: 0.5,
        h_bottom: 0.25,
        ..Default::default()
    };

    let geometry = compute_geometry(&input);
    let flat = compute_flat_geometry(&geometry, &input);
    let expected_svg = to_svg(
        &build_scene(&input, &geometry, &flat),
        &SvgOptions {
            inkscape_layers: input.inkscape_layers,
        },
    );
    assert_eq!(normalize(&stdout_of(SMALL)), normalize(&expected_svg));

    let mut obj_args = SMALL.to_vec();
    obj_args.extend_from_slice(&["--format", "obj"]);
    assert_eq!(
        normalize(&stdout_of(&obj_args)),
        normalize(&geometry_to_obj(&geometry))
    );
}

// ---------------------------------------------------------------------------
// Config merging
// ---------------------------------------------------------------------------

#[test]
fn config_round_trips_through_a_file() {
    let dir = tempdir();
    let path = dir.join("settings.json");

    let saved = stdout_of(&[
        "--format",
        "config",
        "--a",
        "4.25",
        "--projection",
        "spherical",
        "--units",
        "mm",
    ]);
    std::fs::write(&path, &saved).expect("write config");

    let reloaded = stdout_of(&["--config", path.to_str().unwrap(), "--format", "config"]);
    assert_eq!(normalize(&saved), normalize(&reloaded));
}

#[test]
fn flags_override_the_config_file() {
    let dir = tempdir();
    let path = dir.join("settings.json");
    std::fs::write(
        &path,
        stdout_of(&["--format", "config", "--a", "4.25", "--divisions-phi", "12"]),
    )
    .expect("write config");

    let merged = stdout_of(&[
        "--config",
        path.to_str().unwrap(),
        "--format",
        "config",
        "--a",
        "9",
    ]);
    let value: serde_json::Value = serde_json::from_str(&merged).expect("valid JSON");

    assert_eq!(value["a"], 9.0, "flag should win");
    assert_eq!(value["phi_divisions"], 12, "config value should survive");
}

#[test]
fn a_config_with_a_byte_order_mark_still_parses() {
    // PowerShell's `Set-Content -Encoding utf8` writes one, so Windows users
    // hit this immediately otherwise.
    let dir = tempdir();
    let path = dir.join("bom.json");
    std::fs::write(&path, "\u{feff}{\"a\":5.0}").expect("write config");

    let resolved = stdout_of(&["--config", path.to_str().unwrap(), "--format", "config"]);
    let value: serde_json::Value = serde_json::from_str(&resolved).expect("valid JSON");
    assert_eq!(value["a"], 5.0);
}

#[test]
fn a_bad_config_path_fails_cleanly() {
    let out = run(&["--config", "definitely/not/here.json"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("reading"), "unhelpful error: {stderr}");
}

// ---------------------------------------------------------------------------
// Inkscape layer toggle
// ---------------------------------------------------------------------------

#[test]
fn inkscape_layers_default_on_and_can_be_turned_off() {
    assert!(
        stdout_of(&["--divisions-phi", "3", "--divisions-theta", "3"])
            .contains("inkscape:groupmode")
    );

    for off in [
        vec!["--no-inkscape-layers"],
        vec!["--inkscape-layers", "false"],
    ] {
        let mut args = vec!["--divisions-phi", "3", "--divisions-theta", "3"];
        args.extend(off.iter());
        assert!(
            !stdout_of(&args).contains("inkscape:groupmode"),
            "{off:?} did not disable Inkscape layers"
        );
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn invalid_parameters_are_all_reported_at_once() {
    let out = run(&[
        "--theta-min",
        "40",
        "--theta-max",
        "10",
        "--a",
        "-1",
        "--divisions-phi",
        "2",
    ]);
    assert!(!out.status.success(), "should have failed");

    let stderr = String::from_utf8_lossy(&out.stderr);
    for expected in ["a:", "theta_min:", "phi_divisions:"] {
        assert!(stderr.contains(expected), "missing {expected} in: {stderr}");
    }
    assert!(out.stdout.is_empty(), "must not emit a broken pattern");
}

#[test]
fn negative_angles_are_accepted() {
    // `allow_negative_numbers` — otherwise clap rejects these before validation.
    let config = stdout_of(&[
        "--format",
        "config",
        "--theta-min",
        "-90",
        "--theta-max",
        "-20",
    ]);
    let value: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert_eq!(value["theta_min"], -90.0);
    assert_eq!(value["theta_max"], -20.0);
}

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

/// Fields deliberately reachable only through `--config`.
///
/// `cutouts` is a list of surface coordinates placed by clicking in the GUI —
/// there is no sensible flag for it, and it round-trips through the settings
/// file like everything else.
const CONFIG_ONLY_FIELDS: &[&str] = &["cutouts"];

/// Every field of [`EllipsoidInput`] and the flag that sets it.
///
/// Listed explicitly so that adding a field without exposing it fails here.
const FIELD_FLAGS: &[(&str, &str)] = &[
    ("a", "--a"),
    ("b", "--b"),
    ("c", "--c"),
    ("h_top", "--h-top"),
    ("h_middle", "--h-middle"),
    ("h_bottom", "--h-bottom"),
    ("h_top_fraction", "--h-top-fraction"),
    ("h_top_shift", "--h-top-shift"),
    ("phi_divisions", "--divisions-phi"),
    ("theta_divisions", "--divisions-theta"),
    ("theta_min", "--theta-min"),
    ("theta_max", "--theta-max"),
    ("unit", "--units"),
    ("image_offset", "--image-offset"),
    ("min_gap", "--min-gap"),
    ("projection", "--projection"),
    ("inkscape_layers", "--inkscape-layers"),
];

#[test]
fn every_input_field_is_reachable_from_the_command_line() {
    // Serialize a value with a cutout so nothing is skipped by
    // `skip_serializing_if`, or a field could quietly escape this check.
    let populated = EllipsoidInput {
        cutouts: vec![ellipsoid_core::Cutout::hole(0.5, 0.5, 0.125)],
        ..Default::default()
    };
    let json = serde_json::to_value(&populated).expect("serialize");
    let fields: Vec<&str> = json
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();

    for field in &fields {
        assert!(
            FIELD_FLAGS.iter().any(|(f, _)| f == field) || CONFIG_ONLY_FIELDS.contains(field),
            "EllipsoidInput::{field} is unreachable from the CLI; add a flag and list \
             it in FIELD_FLAGS, or record it in CONFIG_ONLY_FIELDS"
        );
    }
    for (field, _) in FIELD_FLAGS {
        assert!(
            fields.contains(field),
            "FIELD_FLAGS lists {field}, which is no longer an EllipsoidInput field"
        );
    }
    for field in CONFIG_ONLY_FIELDS {
        assert!(
            fields.contains(field),
            "CONFIG_ONLY_FIELDS lists {field}, which is no longer an EllipsoidInput field"
        );
    }
}

#[test]
fn cutouts_round_trip_through_a_config_file() {
    let dir = tempdir();
    let path = dir.join("with-cutouts.json");
    std::fs::write(
        &path,
        r#"{"a":3.0,"cutouts":[{"u":0.25,"v":0.5,"diameter":0.125}]}"#,
    )
    .expect("write config");

    let resolved = stdout_of(&["--config", path.to_str().unwrap(), "--format", "config"]);
    let value: serde_json::Value = serde_json::from_str(&resolved).expect("valid JSON");
    assert_eq!(value["cutouts"][0]["u"], 0.25);
    assert_eq!(value["cutouts"][0]["diameter"], 0.125);

    // And they reach the SVG as their own layer.
    let svg = stdout_of(&["--config", path.to_str().unwrap()]);
    assert!(
        svg.contains(r#"id="Cutouts""#),
        "no cutouts layer in the SVG"
    );
}

#[test]
fn help_documents_every_flag() {
    let help = stdout_of(&["--help"]);
    for (_, flag) in FIELD_FLAGS {
        assert!(help.contains(flag), "--help omits {flag}");
    }
    // The negation and the worked examples should be discoverable too.
    assert!(help.contains("--no-inkscape-layers"));
    assert!(help.contains("EXAMPLES:"));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A unique scratch directory under the target dir, so tests never collide.
fn tempdir() -> PathBuf {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("cli-tests");
    // Thread ids are unique within a run; the directory is reused across runs.
    let unique =
        format!("{:?}", std::thread::current().id()).replace(|c: char| !c.is_alphanumeric(), "");
    let dir = base.join(unique);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
