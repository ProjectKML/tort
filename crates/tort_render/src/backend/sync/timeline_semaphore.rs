use std::{ops::Deref, slice};

use ash::{prelude::VkResult, vk};

use crate::backend::{
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TimelineSemaphoreDesc<'a> {
    pub label: Option<&'a str>,
    pub initial_value: u64,
}

pub struct TimelineSemaphore {
    semaphore: vk::Semaphore,
    device: Device,
}

impl TimelineSemaphore {
    pub fn new(device: Device, desc: &TimelineSemaphoreDesc) -> Result<Self, BackendError> {
        let mut semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(desc.initial_value);

        let semaphore = unsafe {
            device.loader().create_semaphore(
                &vk::SemaphoreCreateInfo::default().push_next(&mut semaphore_type_create_info),
                None,
            )
        }?;

        if let Some(label) = desc.label {
            unsafe { debug_utils::set_object_name(&device, semaphore, label) }?;
        }

        Ok(Self { semaphore, device })
    }

    #[inline]
    pub unsafe fn value(&self) -> VkResult<u64> {
        self.device
            .loader()
            .get_semaphore_counter_value(self.semaphore)
    }

    #[inline]
    pub unsafe fn set_value(&self, value: u64) -> VkResult<()> {
        self.device.loader().signal_semaphore(
            &vk::SemaphoreSignalInfo::default()
                .semaphore(self.semaphore)
                .value(value),
        )
    }

    #[inline]
    pub unsafe fn wait_for_value(&self, value: u64, timeout: u64) -> VkResult<()> {
        self.device.loader().wait_semaphores(
            &vk::SemaphoreWaitInfo::default()
                .semaphores(slice::from_ref(&self.semaphore))
                .values(slice::from_ref(&value)),
            timeout,
        )
    }
}

impl Deref for TimelineSemaphore {
    type Target = vk::Semaphore;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.semaphore
    }
}

impl Drop for TimelineSemaphore {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device.loader().destroy_semaphore(self.semaphore, None);
        }
    }
}
