# Changelog

## 2.0.0

Rewritten in Rust. One source tree now builds the desktop app, the web demo and
a headless CLI; the Electron/React/paper.js/three.js implementation is gone.

### Added

- **Command-line tool.** `ellipsoid` generates SVG patterns and OBJ meshes
  without a GUI, reads and writes the same settings JSON as the app, and takes
  every parameter as a flag.
- **Cutouts.** Round holes and free-form shapes, placed by picking on the 3D
  surface or drawn directly on the pattern, then dragged, reshaped or removed.
  They are subtracted from the pattern and from both 3D views. A cutout crossing
  a seam opens that edge, so the two panels form the shape once joined.
- **Two 3D views**, the ellipsoid and the flattened panels, each rendered to a
  texture in its own pane. A **Material** selector switches them between plain
  colour and a UV grid that lands identically on both, so a point on the shell
  can be found on the pattern by eye.
- **Settings persistence.** Settings are remembered between sessions and can be
  saved to and loaded from JSON.
- **Web demo**, the same app compiled to WebAssembly.
- Installers for Windows, macOS and Linux, and prebuilt archives.

### Fixed

Four defects inherited from the JavaScript, each with a test that pins the
change. `RUST_CONVERSION_PLAN.md` §8 records all of them in detail.

- **`thetaMax` was computed from `thetaMin`** (§8.1). It fed the fold decisions
  in both projections, so a partially open top — `0 < thetaMax < 90` — folded the
  `hTop` extension into the panel instead of out beyond it. 11 of 63 test cases
  changed.
- **The spherical cut outline was mirrored** against its own guide lines (§8.7),
  placing them on the wrong side of the outline they belong to.
- **Three or four vertical divisions produced a broken cylindrical layout**
  (§8.8). The unwrap used the acute angle between panels, which is the right one
  only from five divisions up; at three, one panel came out with no width at all
  and the third folded back over the first. Adding a hole to such a pattern also
  hung the app.
- **A scaled or shifted top extension was drawn short** (§8.9) — up to 24% along
  one panel edge — because two of its folds were assumed rather than measured.

### Changed

- Geometry is computed in `f64` throughout.
- Cutout positions are stored as surface coordinates measured by arc length, so
  they stay put when the division counts change.
- Units are written to settings files as `in`, `mm` and `cm`, matching the UI.
  Files written by earlier versions still load.

## 1.0.0

The original Electron application. See the `js-final` tag for its last state.
