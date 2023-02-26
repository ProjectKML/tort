use std::ffi::NulError;

use ash::vk;
use rspirv_reflect::ReflectError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Null byte missing: {0}")]
    Nul(#[from] NulError),
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("Reflection error: {0}")]
    Reflection(#[from] ReflectError),
    #[error("Shaderc error: {0}")]
    Shaderc(#[from] shaderc::Error)
}
