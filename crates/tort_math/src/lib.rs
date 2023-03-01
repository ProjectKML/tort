#![cfg_attr(target_arch = "spirv", no_std)]

#[cfg(not(target_arch = "spirv"))]
pub use bevy_math::*;
#[cfg(target_arch = "spirv")]
pub use spirv_std::glam::*;

mod aabb;

pub use aabb::*;

pub fn dequantize_unorm(value: u32, n: u32) -> f32 {
    let scale = ((1 << n) - 1) as f32;
    value as f32 / scale
}
