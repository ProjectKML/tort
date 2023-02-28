#![cfg_attr(target_arch = "spirv", no_std)]

#[cfg(not(target_arch = "spirv"))]
pub use bevy_math::*;
#[cfg(target_arch = "spirv")]
pub use spirv_std::glam::*;
<<<<<<< HEAD
=======

mod aabb;

pub use aabb::*;

pub fn dequantize_unorm(value: u32, n: u32) -> f32 {
    let scale = ((1i32 << n) - 1i32) as f32;
    value as f32 / scale
}
>>>>>>> b4c5a3a7bd457f283f9813d24b6c3364190628fa
