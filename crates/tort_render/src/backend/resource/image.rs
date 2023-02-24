use std::{borrow::Cow, ops::Deref};

use ash::vk;
use vk_mem_alloc::{
    Allocation, AllocationCreateFlags, AllocationCreateInfo, AllocationInfo, MemoryUsage,
};

use crate::backend::{
    utils::{debug_utils, BackendError, Extent3D},
    Device,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ImageDesc {
    pub label: Option<Cow<'static, str>>,
    pub flags: vk::ImageCreateFlags,
    pub image_type: vk::ImageType,
    pub format: vk::Format,
    pub extent: Extent3D,
    pub mip_levels: u32,
    pub array_layers: u32,
    pub samples: vk::SampleCountFlags,
    pub tiling: vk::ImageTiling,
    pub usage: vk::ImageUsageFlags,
    pub initial_layout: vk::ImageLayout,
    pub allocation_flags: AllocationCreateFlags,
    pub memory_usage: MemoryUsage,
}

pub struct Image {
    image: vk::Image,
    allocation: Allocation,
    allocation_info: AllocationInfo,
    desc: ImageDesc,
    device: Device,
}

impl Image {
    pub fn new(device: Device, desc: &ImageDesc) -> Result<Self, BackendError> {
        let image_create_info = vk::ImageCreateInfo::default()
            .flags(desc.flags)
            .image_type(desc.image_type)
            .format(desc.format)
            .extent(desc.extent.into())
            .mip_levels(desc.mip_levels)
            .array_layers(desc.array_layers)
            .samples(desc.samples)
            .tiling(desc.tiling)
            .usage(desc.usage)
            .initial_layout(desc.initial_layout);

        let allocation_create_info = AllocationCreateInfo {
            flags: desc.allocation_flags,
            usage: desc.memory_usage,
            ..Default::default()
        };

        let (image, allocation, allocation_info) = unsafe {
            vk_mem_alloc::create_image(
                *device.allocator(),
                &image_create_info,
                &allocation_create_info,
            )
        }?;

        if let Some(label) = &desc.label {
            unsafe { debug_utils::set_object_name(&device, image, label) }?;
        }

        Ok(Self {
            image,
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
    pub fn desc(&self) -> &ImageDesc {
        &self.desc
    }
}

impl Deref for Image {
    type Target = vk::Image;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl Drop for Image {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            vk_mem_alloc::destroy_image(*self.device.allocator(), self.image, self.allocation);
        }
    }
}
