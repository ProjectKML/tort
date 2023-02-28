#![cfg_attr(target_arch = "spirv", no_std)]

#[cfg(not(target_arch = "spirv"))]
pub use bevy_math::*;
#[cfg(target_arch = "spirv")]
pub use spirv_std::glam::*;
