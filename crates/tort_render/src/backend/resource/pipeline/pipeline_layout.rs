use std::{borrow::Cow, collections::BTreeMap, ops::Deref, sync::Arc};

use ash::vk;
use rspirv_reflect::{BindingCount, DescriptorType};
use tort_utils::smallvec::SmallVec8;

use crate::backend::{
    resource::{
        descriptor::{
            DescriptorSetLayout, DescriptorSetLayoutBindingDesc, DescriptorSetLayoutDesc,
        },
        pipeline::ShaderModule,
        SamplerDesc,
    },
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PipelineLayoutModifier {
    BindingFlags {
        set: u32,
        binding: u32,
        flags: vk::DescriptorBindingFlags,
    },
    DynamicBuffer {
        set: u32,
        binding: u32,
    },
    ImmutableSamplers {
        set: u32,
        binding: u32,
        immutable_samplers: Vec<SamplerDesc>,
    },
    VariableDescriptorCount {
        set: u32,
        binding: u32,
        descriptor_count: u32,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PushConstantRange {
    pub stage_flags: vk::ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PipelineLayoutDesc {
    pub label: Option<Cow<'static, str>>,
    pub flags: vk::PipelineLayoutCreateFlags,
    pub set_layouts: Vec<DescriptorSetLayoutDesc>,
    pub push_constant_ranges: Vec<PushConstantRange>,
}

fn descriptor_type_from_rspirv(descriptor_type: DescriptorType) -> vk::DescriptorType {
    match descriptor_type {
        DescriptorType::SAMPLER => vk::DescriptorType::SAMPLER,
        DescriptorType::COMBINED_IMAGE_SAMPLER => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        DescriptorType::SAMPLED_IMAGE => vk::DescriptorType::SAMPLED_IMAGE,
        DescriptorType::STORAGE_IMAGE => vk::DescriptorType::STORAGE_IMAGE,
        DescriptorType::UNIFORM_TEXEL_BUFFER => vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
        DescriptorType::STORAGE_TEXEL_BUFFER => vk::DescriptorType::STORAGE_TEXEL_BUFFER,
        DescriptorType::UNIFORM_BUFFER => vk::DescriptorType::UNIFORM_BUFFER,
        DescriptorType::STORAGE_BUFFER => vk::DescriptorType::STORAGE_BUFFER,
        DescriptorType::UNIFORM_BUFFER_DYNAMIC => vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        DescriptorType::STORAGE_BUFFER_DYNAMIC => vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
        DescriptorType::INPUT_ATTACHMENT => vk::DescriptorType::INPUT_ATTACHMENT,

        DescriptorType::INLINE_UNIFORM_BLOCK_EXT => vk::DescriptorType::INLINE_UNIFORM_BLOCK_EXT,
        DescriptorType::ACCELERATION_STRUCTURE_KHR => {
            vk::DescriptorType::ACCELERATION_STRUCTURE_KHR
        }
        DescriptorType::ACCELERATION_STRUCTURE_NV => vk::DescriptorType::ACCELERATION_STRUCTURE_NV,
        _ => panic!("Unknown descriptor type"),
    }
}

impl PipelineLayoutDesc {
    pub fn from_spirv<'a>(
        shader_stages: impl Iterator<Item = (vk::ShaderStageFlags, &'a ShaderModule)>,
        modifiers: &[PipelineLayoutModifier],
    ) -> Self {
        let mut desc = Self::default();
        let mut reflected_sets = BTreeMap::new();

        for (stage_flags, shader_module) in shader_stages {
            for (set_index, set) in shader_module.descriptor_sets() {
                let reflected_set = reflected_sets
                    .entry(*set_index)
                    .or_insert_with(BTreeMap::new);

                for (binding_index, binding) in set {
                    let (reflected_binding, binding_flags) =
                        reflected_set.entry(*binding_index).or_insert_with(|| {
                            (
                                DescriptorSetLayoutBindingDesc::default(),
                                vk::DescriptorBindingFlags::empty(),
                            )
                        });

                    reflected_binding.binding = *binding_index;
                    reflected_binding.descriptor_type = descriptor_type_from_rspirv(binding.ty);
                    reflected_binding.stage_flags |= stage_flags;
                    reflected_binding.descriptor_count = match binding.binding_count {
                        BindingCount::One => 1,
                        BindingCount::StaticSized(size) => size as _,
                        BindingCount::Unbounded => {
                            *binding_flags = vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT;
                            1
                        }
                    };
                }
            }

            if let Some(push_constant_range) = shader_module.push_constant_info() {
                desc.push_constant_ranges.push(PushConstantRange {
                    stage_flags,
                    offset: push_constant_range.offset,
                    size: push_constant_range.size,
                });
            }
        }

        for reflected_set in reflected_sets.values() {
            let mut set_layout_desc = DescriptorSetLayoutDesc::default();
            set_layout_desc.bindings = Vec::with_capacity(reflected_set.len());
            set_layout_desc.binding_flags = Vec::with_capacity(reflected_set.len());

            for (reflected_binding, reflected_binding_flags) in reflected_set.values() {
                set_layout_desc.bindings.push(reflected_binding.clone());
                set_layout_desc.binding_flags.push(*reflected_binding_flags);
            }

            desc.set_layouts.push(set_layout_desc);
        }

        for modifier in modifiers {
            match modifier {
                PipelineLayoutModifier::BindingFlags {
                    set,
                    binding,
                    flags,
                } => desc.set_layouts[*set as usize].binding_flags[*binding as usize] |= *flags,
                PipelineLayoutModifier::DynamicBuffer { set, binding } => {
                    let mut binding =
                        &mut desc.set_layouts[*set as usize].bindings[*binding as usize];
                    binding.descriptor_type =
                        if binding.descriptor_type == vk::DescriptorType::UNIFORM_BUFFER {
                            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC
                        } else if binding.descriptor_type == vk::DescriptorType::STORAGE_BUFFER {
                            vk::DescriptorType::STORAGE_BUFFER_DYNAMIC
                        } else {
                            panic!("Only storage and uniform buffers can be dynamic")
                        }
                }
                PipelineLayoutModifier::ImmutableSamplers {
                    set,
                    binding,
                    immutable_samplers,
                } => {
                    let mut binding =
                        &mut desc.set_layouts[*set as usize].bindings[*binding as usize];

                    assert!(binding.immutable_samplers.is_empty());

                    binding.immutable_samplers = immutable_samplers.clone()
                }
                PipelineLayoutModifier::VariableDescriptorCount {
                    set,
                    binding,
                    descriptor_count,
                } => {
                    desc.set_layouts[*set as usize].bindings[*binding as usize].descriptor_count =
                        *descriptor_count
                }
            }
        }

        desc
    }
}

impl From<&PipelineLayoutDesc> for PipelineLayoutDesc {
    #[inline]
    fn from(desc: &PipelineLayoutDesc) -> Self {
        desc.clone()
    }
}

pub struct PipelineLayout {
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layouts: Vec<Arc<DescriptorSetLayout>>,
    device: Device,
}

impl PipelineLayout {
    pub(crate) fn new(
        device: Device,
        desc: &PipelineLayoutDesc,
        descriptor_set_layout_provider: impl Fn(
            &DescriptorSetLayoutDesc,
        )
            -> Result<Arc<DescriptorSetLayout>, BackendError>,
    ) -> Result<Self, BackendError> {
        let descriptor_set_layouts = desc
            .set_layouts
            .iter()
            .map(descriptor_set_layout_provider)
            .collect::<Result<Vec<_>, _>>()?;

        let descriptor_set_layout_handles = descriptor_set_layouts
            .iter()
            .map(|descriptor_set_layout| ***descriptor_set_layout)
            .collect::<SmallVec8<_>>();

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .flags(desc.flags)
            .set_layouts(&descriptor_set_layout_handles)
            .push_constant_ranges(unsafe {
                tort_utils::slices::cast_unsafe(&desc.push_constant_ranges)
            });

        let pipeline_layout = unsafe {
            device
                .loader()
                .create_pipeline_layout(&pipeline_layout_create_info, None)
        }?;

        if let Some(label) = &desc.label {
            unsafe { debug_utils::set_object_name(&device, pipeline_layout, label) }?;
        }

        Ok(Self {
            pipeline_layout,
            descriptor_set_layouts,
            device,
        })
    }

    #[inline]
    pub fn descriptor_set_layouts(&self) -> &Vec<Arc<DescriptorSetLayout>> {
        &self.descriptor_set_layouts
    }
}

impl Deref for PipelineLayout {
    type Target = vk::PipelineLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.pipeline_layout
    }
}

impl Drop for PipelineLayout {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device
                .loader()
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
