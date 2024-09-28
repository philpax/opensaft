//! Signed distance field function utilities and interpreter

#![cfg_attr(target_arch = "spirv", feature(repr_simd, core_intrinsics))]
#![cfg_attr(target_arch = "spirv", no_std)]

mod opcodes;
pub use opcodes::*;

mod interpreter;
pub use interpreter::*;

mod sdf;
pub use sdf::*;

mod structs;
pub use structs::*;

#[cfg(not(target_arch = "spirv"))]
pub fn get_glsl_sdf_library_code() -> &'static str {
    include_str!("sdf.glsl")
}

#[cfg(not(target_arch = "spirv"))]
pub fn get_glsl_sdf_interpreter_code() -> &'static str {
    include_str!("interpreter.glsl")
}
