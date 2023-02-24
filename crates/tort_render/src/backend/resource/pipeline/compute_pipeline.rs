use std::{borrow::Cow, ffi::CString, iter, slice, sync::Arc};

use ash::vk;
use tort_utils::Uuid;

use crate::backend::{
    resource::pipeline::{
        Pipeline, PipelineLayout, PipelineLayoutDesc, PipelineLayoutModifier, ShaderModule,
        ShaderStageDesc,
    },
    utils::{debug_utils, BackendError},
    Device,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComputePipelineId(Uuid);

impl From<Uuid> for ComputePipelineId {
    #[inline]
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ComputePipelineDesc {
    pub label: Option<Cow<'static, str>>,
    pub flags: vk::PipelineCreateFlags,
    pub stage: ShaderStageDesc,
    pub layout_modifers: Vec<PipelineLayoutModifier>,
}

impl From<&ComputePipelineDesc> for ComputePipelineDesc {
    #[inline]
    fn from(desc: &ComputePipelineDesc) -> Self {
        desc.clone()
    }
}

struct Inner {
    pipeline: vk::Pipeline,
    pipeline_layout: Arc<PipelineLayout>,
    desc: ComputePipelineDesc,
    id: ComputePipelineId,
    device: Device,
}

impl Drop for Inner {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device.loader().destroy_pipeline(self.pipeline, None);
        }
    }
}

#[derive(Clone)]
pub struct ComputePipeline(Arc<Inner>);

impl ComputePipeline {
    pub(crate) fn new(
        device: Device,
        desc: &ComputePipelineDesc,
        id: ComputePipelineId,
        shader_module: &ShaderModule,
        pipeline_layout_provider: impl Fn(
            &PipelineLayoutDesc,
        ) -> Result<Arc<PipelineLayout>, BackendError>,
    ) -> Result<Self, BackendError> {
        let pipeline_layout_desc = PipelineLayoutDesc::from_spirv(
            iter::once((desc.stage.stage, shader_module)),
            &desc.layout_modifers,
        );
        let pipeline_layout = pipeline_layout_provider(&pipeline_layout_desc)?;

        let name = CString::new(&desc.stage.entry_point as &str)?;

        #[allow(unused_assignments)]
        let mut specialization_info = vk::SpecializationInfo::default();

        let compute_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .flags(desc.flags)
            .stage({
                let mut pipeline_shader_stage_create_info =
                    vk::PipelineShaderStageCreateInfo::default()
                        .flags(desc.stage.flags)
                        .stage(vk::ShaderStageFlags::COMPUTE)
                        .module(**shader_module)
                        .name(&name);

                if let Some(spec_info) = &desc.stage.specialization_info {
                    specialization_info = vk::SpecializationInfo::default()
                        .map_entries(unsafe {
                            tort_utils::slices::cast_unsafe(&spec_info.map_entries)
                        })
                        .data(&spec_info.data);

                    pipeline_shader_stage_create_info.p_specialization_info = &specialization_info;
                }

                pipeline_shader_stage_create_info
            })
            .layout(**pipeline_layout);

        let pipeline = unsafe {
            device.loader().create_compute_pipelines(
                vk::PipelineCache::null(),
                slice::from_ref(&compute_pipeline_create_info),
                None,
            )
        }
        .map_err(|(_, result)| result)?[0];

        if let Some(label) = &desc.label {
            unsafe { debug_utils::set_object_name(&device, pipeline, label) }?;
        }

        Ok(Self(Arc::new(Inner {
            pipeline,
            pipeline_layout,
            desc: desc.clone(),
            id,
            device,
        })))
    }

    #[inline]
    pub fn pipeline_layout(&self) -> &Arc<PipelineLayout> {
        &self.0.pipeline_layout
    }

    #[inline]
    pub fn desc(&self) -> &ComputePipelineDesc {
        &self.0.desc
    }

    #[inline]
    pub fn id(&self) -> &ComputePipelineId {
        &self.0.id
    }
}

impl Pipeline for ComputePipeline {
    type Desc = ComputePipelineDesc;
    type Id = ComputePipelineId;
}
