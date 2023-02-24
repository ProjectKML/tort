use std::{borrow::Cow, ops::Deref, sync::Arc};

use ash::vk;
use tort_utils::smallvec::SmallVec8;

use crate::backend::{
    resource::sampler::{Sampler, SamplerDesc},
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayoutBindingDesc {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
    pub immutable_samplers: Vec<SamplerDesc>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayoutDesc {
    pub label: Option<Cow<'static, str>>,
    pub flags: vk::DescriptorSetLayoutCreateFlags,
    pub bindings: Vec<DescriptorSetLayoutBindingDesc>,
    pub binding_flags: Vec<vk::DescriptorBindingFlags>,
}

impl From<&DescriptorSetLayoutDesc> for DescriptorSetLayoutDesc {
    #[inline]
    fn from(desc: &DescriptorSetLayoutDesc) -> Self {
        desc.clone()
    }
}

pub struct DescriptorSetLayout {
    descriptor_set_layout: vk::DescriptorSetLayout,
    immutable_samplers: Vec<Arc<Sampler>>,
    device: Device,
}

impl DescriptorSetLayout {
    pub(crate) fn new(
        device: Device,
        desc: &DescriptorSetLayoutDesc,
        immutable_sampler_provider: impl Fn(&SamplerDesc) -> Result<Arc<Sampler>, BackendError>,
    ) -> Result<Self, BackendError> {
        let immutable_samplers = desc
            .bindings
            .iter()
            .flat_map(|binding_desc| binding_desc.immutable_samplers.iter())
            .map(immutable_sampler_provider)
            .collect::<Result<Vec<_>, _>>()?;

        let immutable_sampler_handles = immutable_samplers
            .iter()
            .map(|sampler| ***sampler)
            .collect::<SmallVec8<_>>();

        let mut sampler_offset = 0;

        let bindings = desc
            .bindings
            .iter()
            .map(|binding_desc| {
                let mut descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::default()
                    .binding(binding_desc.binding)
                    .descriptor_type(binding_desc.descriptor_type)
                    .descriptor_count(binding_desc.descriptor_count)
                    .stage_flags(binding_desc.stage_flags);

                if !binding_desc.immutable_samplers.is_empty() {
                    let new_sampler_offset = sampler_offset + binding_desc.immutable_samplers.len();
                    descriptor_set_layout_binding.descriptor_count =
                        (new_sampler_offset - sampler_offset) as _;
                    descriptor_set_layout_binding.p_immutable_samplers =
                        &immutable_sampler_handles[sampler_offset];

                    sampler_offset = new_sampler_offset;
                }

                descriptor_set_layout_binding
            })
            .collect::<SmallVec8<_>>();

        let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
            .binding_flags(&desc.binding_flags);

        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::default()
            .push_next(&mut binding_flags)
            .flags(desc.flags)
            .bindings(&bindings);

        let descriptor_set_layout = unsafe {
            device
                .loader()
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
        }?;

        if let Some(label) = &desc.label {
            unsafe { debug_utils::set_object_name(&device, descriptor_set_layout, label) }?;
        }

        Ok(Self {
            descriptor_set_layout,
            immutable_samplers,
            device,
        })
    }

    #[inline]
    pub fn immutable_samplers(&self) -> &Vec<Arc<Sampler>> {
        &self.immutable_samplers
    }
}

impl Deref for DescriptorSetLayout {
    type Target = vk::DescriptorSetLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.descriptor_set_layout
    }
}

impl Drop for DescriptorSetLayout {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device
                .loader()
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}
