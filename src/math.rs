//! Types extracted from `macaw` and retargeted towards base `glam`.
//!
//! An `openmacaw` may be created at some point, but this carries
//! the code used purely for this library.

mod bounding_box;
pub use bounding_box::*;

mod ray3;
pub use ray3::*;

mod plane3;
pub use plane3::*;
