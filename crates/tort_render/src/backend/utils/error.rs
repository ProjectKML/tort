use std::ffi::NulError;

use ash::vk;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Null byte missing: {0}")]
    Nul(#[from] NulError),
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}
