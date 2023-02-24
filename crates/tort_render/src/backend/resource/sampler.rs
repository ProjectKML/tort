use std::{borrow::Cow, ops::Deref};

use ash::vk;
use tort_utils::OrderedFloat;

use crate::backend::{
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SamplerDesc {
    pub label: Option<Cow<'static, str>>,
    pub flags: vk::SamplerCreateFlags,
    pub mag_filter: vk::Filter,
    pub min_filter: vk::Filter,
    pub mipmap_mode: vk::SamplerMipmapMode,
    pub address_mode_u: vk::SamplerAddressMode,
    pub address_mode_v: vk::SamplerAddressMode,
    pub address_mode_w: vk::SamplerAddressMode,
    pub mip_lod_bias: OrderedFloat<f32>,
    pub anisotropy_enable: bool,
    pub max_anisotropy: OrderedFloat<f32>,
    pub compare_enable: bool,
    pub compare_op: vk::CompareOp,
    pub min_lod: OrderedFloat<f32>,
    pub max_lod: OrderedFloat<f32>,
    pub border_color: vk::BorderColor,
    pub unnormalized_coordinates: bool,
}

impl From<&SamplerDesc> for SamplerDesc {
    #[inline]
    fn from(desc: &SamplerDesc) -> Self {
        desc.clone()
    }
}

pub struct Sampler {
    sampler: vk::Sampler,
    device: Device,
}

impl Sampler {
    pub fn new(device: Device, desc: &SamplerDesc) -> Result<Self, BackendError> {
        let sampler = unsafe {
            device.loader().create_sampler(
                &vk::SamplerCreateInfo::default()
                    .flags(desc.flags)
                    .mag_filter(desc.mag_filter)
                    .min_filter(desc.min_filter)
                    .mipmap_mode(desc.mipmap_mode)
                    .address_mode_u(desc.address_mode_u)
                    .address_mode_v(desc.address_mode_v)
                    .address_mode_w(desc.address_mode_w)
                    .mip_lod_bias(*desc.mip_lod_bias)
                    .anisotropy_enable(desc.anisotropy_enable)
                    .max_anisotropy(*desc.max_anisotropy)
                    .compare_enable(desc.compare_enable)
                    .compare_op(desc.compare_op)
                    .min_lod(*desc.min_lod)
                    .max_lod(*desc.max_lod)
                    .border_color(desc.border_color)
                    .unnormalized_coordinates(desc.unnormalized_coordinates),
                None,
            )?
        };

        if let Some(label) = &desc.label {
            unsafe { debug_utils::set_object_name(&device, sampler, label) }?;
        }

        Ok(Self { sampler, device })
    }
}

impl Deref for Sampler {
    type Target = vk::Sampler;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.sampler
    }
}

impl Drop for Sampler {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device.loader().destroy_sampler(self.sampler, None);
        }
    }
}
