use std::{ops::Deref, sync::Arc};

use ash::vk;

use crate::backend::{
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CommandPoolDesc<'a> {
    pub label: Option<&'a str>,
    pub flags: vk::CommandPoolCreateFlags,
    pub family_index: u32,
}

struct Inner {
    command_pool: vk::CommandPool,
    device: Device,
}

impl Drop for Inner {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device
                .loader()
                .destroy_command_pool(self.command_pool, None);
        }
    }
}

#[derive(Clone)]
pub struct CommandPool(Arc<Inner>);

impl CommandPool {
    pub fn new(device: Device, desc: &CommandPoolDesc) -> Result<Self, BackendError> {
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(desc.flags)
            .queue_family_index(desc.family_index);

        let command_pool = unsafe {
            device
                .loader()
                .create_command_pool(&command_pool_create_info, None)
        }?;

        if let Some(label) = desc.label {
            unsafe { debug_utils::set_object_name(&device, command_pool, label) }?;
        }

        Ok(Self(Arc::new(Inner {
            command_pool,
            device,
        })))
    }
}

impl Deref for CommandPool {
    type Target = vk::CommandPool;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.command_pool
    }
}
