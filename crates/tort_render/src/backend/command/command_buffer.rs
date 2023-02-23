use std::{ops::Deref, slice, sync::Arc};

use ash::vk;

use crate::backend::{
    command::CommandPool,
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CommandBufferDesc<'a> {
    pub label: Option<&'a str>,
}

struct Inner {
    command_buffer: vk::CommandBuffer,
    command_pool: CommandPool,
    device: Device,
}

impl Drop for Inner {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device
                .loader()
                .free_command_buffers(*self.command_pool, slice::from_ref(&self.command_buffer))
        }
    }
}

#[derive(Clone)]
pub struct CommandBuffer(Arc<Inner>);

impl CommandBuffer {
    pub fn new(
        device: Device,
        command_pool: CommandPool,
        desc: &CommandBufferDesc,
    ) -> Result<Self, BackendError> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(*command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            device
                .loader()
                .allocate_command_buffers(&command_buffer_allocate_info)
        }?[0];

        if let Some(label) = desc.label {
            unsafe { debug_utils::set_object_name(&device, command_buffer, label) }?;
        }

        Ok(Self(Arc::new(Inner {
            command_buffer,
            command_pool,
            device,
        })))
    }
}

impl Deref for CommandBuffer {
    type Target = vk::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.0.command_buffer
    }
}
