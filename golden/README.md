# `golden/`

Recorded output for a matrix of settings, used as a regression check. If a change to the geometry
pipeline moves a number, it shows up here.

Two parallel sets, 63 cases each, sharing the same names:

| | |
| --- | --- |
| `<case>.json` | the case definition **plus** the geometry, flattening and OBJ it produces |
| `svg/<case>.svg` | the flat pattern rendered from that same case |

## What one file holds

```
name      "h_none_cylindrical"
settings  the input — a, b, c, hTop, hMiddle, hBottom, hTopFraction,
          hTopShift, Divisions, divisions, thetaMin, thetaMax, projection
geometry  phi_divisions, theta_divisions, widest_row, points[9][19]
flat      widest_row, edges_flat[8 strips][19 rungs][2 ends]
obj       surface and flat, as OBJ text
```

Only `settings` is input; the rest is recorded output. Adding a case means writing a file with
nothing but a `settings` block and regenerating.

**`Divisions` and `divisions` are different fields.** The names come from the original
JavaScript, which had two settings differing only in case, and the fixtures kept them so the
matrix did not have to be retyped when they stopped being a parity harness:

- `Divisions` (capital) → `phi_divisions`, around the circumference
- `divisions` (lowercase) → `theta_divisions`, top to bottom

The Rust structs rename both, so this only matters if you hand-edit a fixture.

`settings` carries no unit, offset, `min_gap`, layer or cutout fields. Each consumer supplies
those itself.

## The matrix

Chosen to reach the branches rather than the defaults. Most cases exist in both projections.

| Prefix | Varies |
| --- | --- |
| `shape_*` | the axis ratios — sphere, oblate, prolate, closed |
| `h_*` | the inserted heights, including a scaled and shifted top |
| `theta_<projection>_<min>_<max>` | the angle limits; `m` is minus, so `m35_45` is thetaMin −35, thetaMax 45 |
| `div_*` | the division counts, including the minimum of three |
| `draw_*` | drawing options — units, image offset, `min_gap` |
| `ref_screenshot` | the settings from the original project's screenshot |

## Who reads them

**`crates/ellipsoid-core/tests/snapshots.rs`** — the only consumer that reads the recorded
numbers. Recomputes everything from `settings` and compares at a tolerance of `1e-12`, which is
there only because a compiler or libm change may move the last place or two. Asserts it found at
least 50 files, so a deleted matrix cannot pass silently. The OBJ blocks are parsed and compared
as structure — face topology and vertex counts — rather than diffed as text.

**`crates/ellipsoid-core/tests/invariants.rs`** — reads only `settings` and ignores every
recorded number. It asks whether the output is *right* rather than *unchanged*: that flattening
preserves lengths, that the pattern lies in one plane, that cylindrical gores sit side by side,
that nothing is drawn shorter than it is. It also generates a denser sweep on top of this matrix
(phi 3..13 × theta 3/4/7/16), which is what caught §8.8 and §8.9 in the plan.

**`crates/ellipsoid-pattern/tests/snapshots.rs`** — `svg_matches_the_snapshots` reads only
`settings`, supplies its own drawing options (inches, `image_offset` 0.5, `min_gap` 0.001,
Inkscape layers, **no cutouts**), renders, and compares the SVG as text. The other two tests in
that file build their own input and do not touch this folder.

## Regenerating

```sh
UPDATE_GOLDEN=1 cargo test -p ellipsoid-core     # the .json files
UPDATE_GOLDEN=1 cargo test -p ellipsoid-pattern  # the .svg files
```

**Regenerate deliberately, after reading the diff — never to turn a red test green.** The point
of the arrangement is that the diff is a reviewable claim about blast radius. When plan §8.8
changed the three-strip unwrap, exactly one case moved; when §8.9 fixed the shaped top extension,
exactly four did.

A related habit: a change can shift other cases by *less* than the tolerance. The test passes
either way, but `UPDATE_GOLDEN` still rewrites those files. Revert them. Eight files of
last-digit churn will bury the four that matter.

## Why they are self-snapshots

**A snapshot pins behaviour; it cannot judge it.** `div_min_cylindrical` faithfully recorded a
broken three-strip layout for as long as it existed, and the CLI's own snapshots recorded the same
thing — both were green throughout. Only the invariants could say the numbers were wrong. The two
kinds of test fail in usefully different ways, and the geometry needs both.
