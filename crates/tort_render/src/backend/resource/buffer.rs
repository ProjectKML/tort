use std::{borrow::Cow, ops::Deref};

use ash::vk;
use vk_mem_alloc::{
    Allocation, AllocationCreateFlags, AllocationCreateInfo, AllocationInfo, MemoryUsage,
};

use crate::backend::{
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BufferDesc {
    pub label: Option<Cow<'static, str>>,
    pub flags: vk::BufferCreateFlags,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub allocation_flags: AllocationCreateFlags,
    pub memory_usage: MemoryUsage,
}

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Allocation,
    allocation_info: AllocationInfo,
    desc: BufferDesc,
    device: Device,
}

impl Buffer {
    pub fn new(device: Device, desc: &BufferDesc) -> Result<Self, BackendError> {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .flags(desc.flags)
            .size(desc.size)
            .usage(desc.usage);

        let allocation_create_info = AllocationCreateInfo {
            flags: desc.allocation_flags,
            usage: desc.memory_usage,
            ..Default::default()
        };

        let (buffer, allocation, allocation_info) = unsafe {
            vk_mem_alloc::create_buffer(
                *device.allocator(),
                &buffer_create_info,
                &allocation_create_info,
            )
        }?;

        if let Some(label) = &desc.label {
            unsafe { debug_utils::set_object_name(&device, buffer, label) }?;
        }

        Ok(Self {
            buffer,
            allocation,
            allocation_info,
            desc: desc.clone(),
            device,
        })
    }

    #[inline]
    pub fn allocation(&self) -> &Allocation {
        &self.allocation
    }

    #[inline]
    pub fn allocation_info(&self) -> &AllocationInfo {
        &self.allocation_info
    }

    #[inline]
    pub fn desc(&self) -> &BufferDesc {
        &self.desc
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl Drop for Buffer {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            vk_mem_alloc::destroy_buffer(*self.device.allocator(), self.buffer, self.allocation);
        }
    }
}
