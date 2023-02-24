use std::{collections::BTreeMap, ops::Deref};

use ash::vk;
use rspirv_reflect::{DescriptorInfo, PushConstantInfo, Reflection};

use crate::backend::{
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ShaderModuleDesc<'a> {
    pub label: Option<&'a str>,
    pub flags: vk::ShaderModuleCreateFlags,
    pub code: &'a [u32],
}

pub struct ShaderModule {
    shader_module: vk::ShaderModule,
    descriptor_sets: BTreeMap<u32, BTreeMap<u32, DescriptorInfo>>,
    push_constant_info: Option<PushConstantInfo>,
    device: Device,
}

impl ShaderModule {
    pub fn new(device: Device, desc: &ShaderModuleDesc) -> Result<Self, BackendError> {
        let shader_module = unsafe {
            device.loader().create_shader_module(
                &vk::ShaderModuleCreateInfo::default()
                    .flags(desc.flags)
                    .code(desc.code),
                None,
            )
        }?;

        if let Some(label) = desc.label {
            unsafe { debug_utils::set_object_name(&device, shader_module, label) }?;
        }

        let reflection = Reflection::new_from_spirv(tort_utils::slices::bytes_of(desc.code))?;

        Ok(Self {
            shader_module,
            descriptor_sets: reflection.get_descriptor_sets()?,
            push_constant_info: reflection.get_push_constant_range()?,
            device,
        })
    }

    #[inline]
    pub fn descriptor_sets(&self) -> &BTreeMap<u32, BTreeMap<u32, DescriptorInfo>> {
        &self.descriptor_sets
    }

    #[inline]
    pub fn push_constant_info(&self) -> &Option<PushConstantInfo> {
        &self.push_constant_info
    }
}

impl Deref for ShaderModule {
    type Target = vk::ShaderModule;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.shader_module
    }
}

impl Drop for ShaderModule {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device
                .loader()
                .destroy_shader_module(self.shader_module, None);
        }
    }
}
