//! Ellipsoid geometry generation and flat-pattern unrolling.
//!
//! This crate is the port of `app/utils/ellipsoid.js` and
//! `app/utils/geometryHelpers.js`. It is deliberately free of UI and I/O
//! dependencies so it can be driven by the GUI, the CLI, and tests alike.
//!
//! # Where the JavaScript went
//!
//! Modules throughout this crate name the function each one came from. Those
//! paths no longer exist in the working tree — the original was deleted in
//! Phase 9 — but they are all in git, at the tag **`js-final`**:
//!
//! ```text
//! git show js-final:app/utils/ellipsoid.js
//! git checkout js-final -- app/utils    # to regenerate the golden files
//! ```
//!
//! The `golden/` files are the part that still matters day to day: they are
//! the JavaScript's *answers*, checked in, and `tests/parity.rs` holds this
//! crate to them at a `1e-9` relative tolerance. That tolerance is why all the
//! math here is `f64` ([`glam::DVec3`]) — `f32` cannot reach it. Conversion to
//! `f32` happens only where meshes are handed to the renderer.
//!
//! The port is deliberately bug-for-bug faithful — see `RUST_CONVERSION_PLAN.md`
//! §8 for the list of known defects carried over on purpose, the most
//! consequential being the `theta_max` mix-up in [`flatten`].

pub mod flatten;
pub mod geometry;
pub mod input;
pub mod obj;
pub mod rotate;
pub mod surface;
pub mod units;

pub use flatten::{FlatGeometry, compute_flat_geometry};
pub use geometry::{Geometry, compute_geometry};
pub use input::{EllipsoidInput, Projection};
pub use obj::{flat_to_obj, geometry_to_obj};
pub use surface::{Cutout, SurfaceHit, SurfaceParam, flat_point, ray_hit, surface_point};
pub use units::Unit;

/// Re-exported so downstream crates share one vector type.
pub use glam::{DVec2, DVec3};
