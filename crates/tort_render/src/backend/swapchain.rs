use std::ops::Deref;

use anyhow::Result;
use ash::{prelude::VkResult, vk};
use tort_ecs::{self as bevy_ecs, system::Resource};
use tort_window::PresentMode;

use crate::backend::{Device, Instance, Surface};

pub struct SurfaceCapabilities {
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
}

impl SurfaceCapabilities {
    #[inline]
    pub unsafe fn new(
        instance: &Instance,
        device: &Device,
        surface_info: &vk::PhysicalDeviceSurfaceInfo2KHR,
    ) -> VkResult<Self> {
        Ok(Self {
            surface_capabilities: instance
                .get_surface_capabilities2_loader()
                .get_physical_device_surface_capabilities2(*device.physical_device(), surface_info)?
                .surface_capabilities,
        })
    }
}

pub struct SurfaceFormats {
    pub supported_formats: Vec<vk::SurfaceFormatKHR>,
}

impl SurfaceFormats {
    #[inline]
    pub unsafe fn new(
        instance: &Instance,
        device: &Device,
        surface_info: &vk::PhysicalDeviceSurfaceInfo2KHR,
    ) -> VkResult<Self> {
        let get_surface_capabilities2_loader = instance.get_surface_capabilities2_loader();
        let physical_device = *device.physical_device();

        let mut supported_formats: Vec<_> = (0..get_surface_capabilities2_loader
            .get_physical_device_surface_formats2_len(physical_device, surface_info)?)
            .map(|_| vk::SurfaceFormat2KHR::default())
            .collect();
        get_surface_capabilities2_loader.get_physical_device_surface_formats2(
            physical_device,
            surface_info,
            &mut supported_formats,
        )?;

        Ok(Self {
            supported_formats: supported_formats
                .iter()
                .map(|format| format.surface_format)
                .collect(),
        })
    }

    #[inline]
    pub fn find_ldr_format(&self) -> Option<vk::SurfaceFormatKHR> {
        const FORMATS: [vk::Format; 4] = [
            vk::Format::R8G8B8A8_SRGB,
            vk::Format::B8G8R8A8_SRGB,
            vk::Format::R8G8B8A8_UNORM,
            vk::Format::B8G8R8A8_UNORM,
        ];

        self.supported_formats
            .iter()
            .find(|f| FORMATS.contains(&f.format))
            .copied()
    }

    #[inline]
    pub fn find_hdr_format(&self) -> Option<vk::SurfaceFormatKHR> {
        self.supported_formats
            .iter()
            .find(|f| {
                f.format == vk::Format::R16G16B16A16_SFLOAT
                    && f.color_space == vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT
            })
            .copied()
    }
}

#[derive(Resource)]
pub struct Swapchain {
    surface_capabilities: SurfaceCapabilities,

    surface_formats: SurfaceFormats,
    present_modes: Vec<vk::PresentModeKHR>,

    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,

    used_surface_format: vk::SurfaceFormatKHR,
    used_present_mode: vk::PresentModeKHR,
    requested_present_mode: PresentMode,

    swapchain: vk::SwapchainKHR,

    _instance: Instance,
    _surface: Surface,
    device: Device,
}

impl Swapchain {
    pub fn new(
        instance: Instance,
        surface: Surface,
        device: Device,
        requested_present_mode: PresentMode,
        old_swapchain: Option<&Self>,
    ) -> Result<Self> {
        let device_loader = device.loader();

        let surface_info = vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(*surface.surface());

        let surface_capabilities =
            unsafe { SurfaceCapabilities::new(&instance, &device, &surface_info) }?;
        let surface_formats = unsafe { SurfaceFormats::new(&instance, &device, &surface_info) }?;
        let present_modes = unsafe {
            instance
                .surface_loader()
                .get_physical_device_surface_present_modes(
                    *device.physical_device(),
                    surface_info.surface,
                )
        }?;

        let get_present_mode_if_supported = |present_mode: vk::PresentModeKHR| {
            present_modes.iter().find(|p| **p == present_mode).copied()
        };

        let used_surface_format = surface_formats
            .find_hdr_format()
            .or_else(|| surface_formats.find_ldr_format())
            .ok_or_else(|| anyhow::anyhow!("Failed to find surface format"))?;

        let used_present_mode = match requested_present_mode {
            PresentMode::AutoVsync => {
                get_present_mode_if_supported(vk::PresentModeKHR::FIFO_RELAXED)
                    .unwrap_or(vk::PresentModeKHR::FIFO)
            }
            PresentMode::AutoNoVsync => {
                get_present_mode_if_supported(vk::PresentModeKHR::IMMEDIATE)
                    .or_else(|| get_present_mode_if_supported(vk::PresentModeKHR::MAILBOX))
                    .unwrap_or(vk::PresentModeKHR::FIFO)
            }
            PresentMode::Immediate => {
                get_present_mode_if_supported(vk::PresentModeKHR::IMMEDIATE).unwrap()
            }
            PresentMode::Mailbox => {
                get_present_mode_if_supported(vk::PresentModeKHR::MAILBOX).unwrap()
            }
            PresentMode::Fifo => vk::PresentModeKHR::FIFO,
        };

        let min_image_count = 3.max(surface_capabilities.surface_capabilities.min_image_count);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface_info.surface)
            .min_image_count(min_image_count)
            .image_format(used_surface_format.format)
            .image_color_space(used_surface_format.color_space)
            .image_extent(surface_capabilities.surface_capabilities.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(used_present_mode)
            .old_swapchain(old_swapchain.map(|sc| sc.swapchain).unwrap_or_default());

        let swapchain_loader = device.swapchain_loader();
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }?;

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }?;
        let image_views = images
            .iter()
            .map(|image| {
                let image_view_create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain_create_info.image_format)
                    .components(Default::default())
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .level_count(1)
                            .layer_count(1),
                    );

                unsafe { device_loader.create_image_view(&image_view_create_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            surface_capabilities,

            surface_formats,
            present_modes,

            images,
            image_views,

            used_surface_format,
            used_present_mode,
            requested_present_mode,

            swapchain,

            _instance: instance,
            device,
            _surface: surface,
        })
    }

    #[inline]
    pub fn surface_capabilities(&self) -> &SurfaceCapabilities {
        &self.surface_capabilities
    }

    #[inline]
    pub fn surface_formats(&self) -> &SurfaceFormats {
        &self.surface_formats
    }

    #[inline]
    pub fn present_modes(&self) -> &Vec<vk::PresentModeKHR> {
        &self.present_modes
    }

    #[inline]
    pub fn images(&self) -> &Vec<vk::Image> {
        &self.images
    }

    #[inline]
    pub fn image_views(&self) -> &Vec<vk::ImageView> {
        &self.image_views
    }

    #[inline]
    pub fn used_surface_format(&self) -> vk::SurfaceFormatKHR {
        self.used_surface_format
    }

    #[inline]
    pub fn used_present_mode(&self) -> vk::PresentModeKHR {
        self.used_present_mode
    }

    #[inline]
    pub fn requested_present_mode(&self) -> PresentMode {
        self.requested_present_mode
    }

    #[inline]
    pub fn swapchain(&self) -> &vk::SwapchainKHR {
        &self.swapchain
    }
}

impl Deref for Swapchain {
    type Target = vk::SwapchainKHR;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}

impl Drop for Swapchain {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let device_loader = self.device.loader();

            self.image_views
                .iter()
                .for_each(|image_view| device_loader.destroy_image_view(*image_view, None));

            self.device
                .swapchain_loader()
                .destroy_swapchain(self.swapchain, None);
        }
    }
}
