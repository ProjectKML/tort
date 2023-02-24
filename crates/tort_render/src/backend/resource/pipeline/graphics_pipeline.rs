use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    ops::Deref,
    slice,
    sync::Arc,
};

use ash::vk;
use tort_utils::{
    smallvec::{SmallVec4, SmallVec8},
    OrderedFloat, Uuid,
};

use crate::backend::{
    resource::pipeline::{
        Pipeline, PipelineLayout, PipelineLayoutDesc, PipelineLayoutModifier, ShaderModule,
        ShaderStageDesc,
    },
    utils::{debug_utils, BackendError, Rect2D},
    Device,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineId(Uuid);

impl From<Uuid> for GraphicsPipelineId {
    #[inline]
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineDesc {
    pub label: Option<Cow<'static, str>>,
    pub flags: vk::PipelineCreateFlags,
    pub stages: Vec<ShaderStageDesc>,
    pub vertex_input_state: Option<VertexInputStateDesc>,
    pub input_assembly_state: InputAssemblyStateDesc,
    pub viewport_state: ViewportStateDesc,
    pub rasterization_state: RasterizationStateDesc,
    pub multisample_state: MultisampleStateDesc,
    pub depth_stencil_state: Option<DepthStencilStateDesc>,
    pub color_blend_state: ColorBlendStateDesc,
    pub dynamic_state: DynamicStateDesc,
    pub rendering_state: RenderingStateDesc,
    pub layout_modifiers: Vec<PipelineLayoutModifier>,
}

impl From<&GraphicsPipelineDesc> for GraphicsPipelineDesc {
    #[inline]
    fn from(desc: &GraphicsPipelineDesc) -> Self {
        desc.clone()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexInputBindingDesc {
    pub binding: u32,
    pub stride: u32,
    pub input_rate: vk::VertexInputRate,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexInputAttributeDesc {
    pub location: u32,
    pub binding: u32,
    pub format: vk::Format,
    pub offset: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct VertexInputStateDesc {
    pub flags: vk::PipelineVertexInputStateCreateFlags,
    pub bindings: Vec<VertexInputBindingDesc>,
    pub attributes: Vec<VertexInputAttributeDesc>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct InputAssemblyStateDesc {
    pub flags: vk::PipelineInputAssemblyStateCreateFlags,
    pub topology: vk::PrimitiveTopology,
    pub primitive_restart_enable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Viewport {
    pub x: OrderedFloat<f32>,
    pub y: OrderedFloat<f32>,
    pub width: OrderedFloat<f32>,
    pub height: OrderedFloat<f32>,
    pub min_depth: OrderedFloat<f32>,
    pub max_depth: OrderedFloat<f32>,
}

impl Viewport {
    #[inline]
    pub fn new(x: f32, y: f32, width: f32, height: f32, min_depth: f32, max_depth: f32) -> Self {
        Self {
            x: OrderedFloat(x),
            y: OrderedFloat(y),
            width: OrderedFloat(width),
            height: OrderedFloat(height),
            min_depth: OrderedFloat(min_depth),
            max_depth: OrderedFloat(max_depth),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ViewportStateDesc {
    pub flags: vk::PipelineViewportStateCreateFlags,
    pub viewports: Vec<Viewport>,
    pub scissors: Vec<Rect2D>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RasterizationStateDesc {
    pub flags: vk::PipelineRasterizationStateCreateFlags,
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: vk::PolygonMode,
    pub cull_mode: vk::CullModeFlags,
    pub front_face: vk::FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: OrderedFloat<f32>,
    pub depth_bias_clamp: OrderedFloat<f32>,
    pub depth_bias_slope_factor: OrderedFloat<f32>,
    pub line_width: OrderedFloat<f32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MultisampleStateDesc {
    pub flags: vk::PipelineMultisampleStateCreateFlags,
    pub rasterization_samples: vk::SampleCountFlags,
    pub sample_shading_enable: bool,
    pub min_sample_shading: OrderedFloat<f32>,
    pub sample_mask: Vec<vk::SampleMask>,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StencilOpState {
    pub fail_op: vk::StencilOp,
    pub pass_op: vk::StencilOp,
    pub depth_fail_op: vk::StencilOp,
    pub compare_op: vk::CompareOp,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DepthStencilStateDesc {
    pub flags: vk::PipelineDepthStencilStateCreateFlags,
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: vk::CompareOp,
    pub depth_bounds_test_enable: bool,
    pub stencil_test_enable: bool,
    pub front: StencilOpState,
    pub back: StencilOpState,
    pub min_depth_bounds: OrderedFloat<f32>,
    pub max_depth_bounds: OrderedFloat<f32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColorBlendAttachmentState {
    pub blend_enable: bool,
    pub src_color_blend_factor: vk::BlendFactor,
    pub dst_color_blend_factor: vk::BlendFactor,
    pub color_blend_op: vk::BlendOp,
    pub src_alpha_blend_factor: vk::BlendFactor,
    pub dst_alpha_blend_factor: vk::BlendFactor,
    pub alpha_blend_op: vk::BlendOp,
    pub color_write_mask: vk::ColorComponentFlags,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColorBlendStateDesc {
    pub flags: vk::PipelineColorBlendStateCreateFlags,
    pub logic_op_enable: bool,
    pub logic_op: vk::LogicOp,
    pub attachments: Vec<ColorBlendAttachmentState>,
    pub blend_constants: [OrderedFloat<f32>; 4],
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DynamicStateDesc {
    pub flags: vk::PipelineDynamicStateCreateFlags,
    pub dynamic_states: Vec<vk::DynamicState>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RenderingStateDesc {
    pub color_attachment_formats: Vec<vk::Format>,
    pub depth_attachment_format: vk::Format,
    pub stencil_attachment_format: vk::Format,
    pub view_mask: u32,
}

struct Inner {
    pipeline: vk::Pipeline,
    pipeline_layout: Arc<PipelineLayout>,
    desc: GraphicsPipelineDesc,
    id: GraphicsPipelineId,
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
pub struct GraphicsPipeline(Arc<Inner>);

impl GraphicsPipeline {
    pub(crate) fn new(
        device: Device,
        desc: &GraphicsPipelineDesc,
        id: GraphicsPipelineId,
        shader_modules: &[Arc<ShaderModule>],
        pipeline_layout_provider: impl Fn(
            &PipelineLayoutDesc,
        ) -> Result<Arc<PipelineLayout>, BackendError>,
    ) -> Result<Self, BackendError> {
        let pipeline_layout_desc = PipelineLayoutDesc::from_spirv(
            desc.stages.iter().map(|stage_desc| stage_desc.stage).zip(
                shader_modules
                    .iter()
                    .map(|shader_module| shader_module.deref()),
            ),
            &desc.layout_modifiers,
        );
        let pipeline_layout = pipeline_layout_provider(&pipeline_layout_desc)?;

        let num_stages = desc.stages.len();

        let mut names = SmallVec8::with_capacity(num_stages);
        let mut specialization_infos = SmallVec8::with_capacity(num_stages);

        let mut pipeline_shader_stage_create_infos = SmallVec4::with_capacity(num_stages);

        for (i, stage_desc) in desc.stages.iter().enumerate() {
            let name = CString::new(&stage_desc.entry_point as &str)?;

            let mut pipeline_shader_stage_create_info =
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(stage_desc.stage)
                    .module(**shader_modules[i])
                    .name(unsafe { CStr::from_ptr(name.as_ptr()) });

            names.push(name);

            if let Some(spec_info) = &stage_desc.specialization_info {
                specialization_infos.push(
                    vk::SpecializationInfo::default()
                        .map_entries(unsafe {
                            tort_utils::slices::cast_unsafe(&spec_info.map_entries)
                        })
                        .data(&spec_info.data),
                );
                pipeline_shader_stage_create_info.p_specialization_info =
                    specialization_infos.last().unwrap();
            }

            pipeline_shader_stage_create_infos.push(pipeline_shader_stage_create_info);
        }

        #[allow(unused_assignments)]
        let mut pipeline_vertex_input_state_create_info =
            vk::PipelineVertexInputStateCreateInfo::default();

        let pipeline_input_assembly_state_create_info =
            vk::PipelineInputAssemblyStateCreateInfo::default()
                .flags(desc.input_assembly_state.flags)
                .topology(desc.input_assembly_state.topology)
                .primitive_restart_enable(desc.input_assembly_state.primitive_restart_enable);

        let pipeline_viewport_state_create_info = vk::PipelineViewportStateCreateInfo::default()
            .flags(desc.viewport_state.flags)
            .viewports(unsafe { tort_utils::slices::cast_unsafe(&desc.viewport_state.viewports) })
            .scissors(unsafe { tort_utils::slices::cast_unsafe(&desc.viewport_state.scissors) });

        let pipeline_rasterization_state_create_info =
            vk::PipelineRasterizationStateCreateInfo::default()
                .flags(desc.rasterization_state.flags)
                .depth_clamp_enable(desc.rasterization_state.depth_clamp_enable)
                .rasterizer_discard_enable(desc.rasterization_state.rasterizer_discard_enable)
                .polygon_mode(desc.rasterization_state.polygon_mode)
                .cull_mode(desc.rasterization_state.cull_mode)
                .front_face(desc.rasterization_state.front_face)
                .depth_bias_enable(desc.rasterization_state.depth_bias_enable)
                .depth_bias_constant_factor(desc.rasterization_state.depth_bias_constant_factor.0)
                .depth_bias_clamp(desc.rasterization_state.depth_bias_clamp.0)
                .depth_bias_slope_factor(desc.rasterization_state.depth_bias_slope_factor.0)
                .line_width(desc.rasterization_state.line_width.0);

        let pipeline_multisample_state_create_info =
            vk::PipelineMultisampleStateCreateInfo::default()
                .flags(desc.multisample_state.flags)
                .rasterization_samples(desc.multisample_state.rasterization_samples)
                .sample_shading_enable(desc.multisample_state.sample_shading_enable)
                .min_sample_shading(desc.multisample_state.min_sample_shading.0)
                .sample_mask(&desc.multisample_state.sample_mask)
                .alpha_to_coverage_enable(desc.multisample_state.alpha_to_coverage_enable)
                .alpha_to_one_enable(desc.multisample_state.alpha_to_one_enable);

        #[allow(unused_assignments)]
        let mut pipeline_depth_stencil_state_create_info =
            vk::PipelineDepthStencilStateCreateInfo::default();

        let color_blend_attachments = desc
            .color_blend_state
            .attachments
            .iter()
            .map(|attachment_desc| {
                vk::PipelineColorBlendAttachmentState::default()
                    .blend_enable(attachment_desc.blend_enable)
                    .src_color_blend_factor(attachment_desc.src_color_blend_factor)
                    .dst_color_blend_factor(attachment_desc.dst_color_blend_factor)
                    .color_blend_op(attachment_desc.color_blend_op)
                    .src_alpha_blend_factor(attachment_desc.src_alpha_blend_factor)
                    .dst_alpha_blend_factor(attachment_desc.dst_alpha_blend_factor)
                    .alpha_blend_op(attachment_desc.alpha_blend_op)
                    .color_write_mask(attachment_desc.color_write_mask)
            })
            .collect::<SmallVec8<_>>();

        let pipeline_color_blend_state_create_info =
            vk::PipelineColorBlendStateCreateInfo::default()
                .flags(desc.color_blend_state.flags)
                .logic_op_enable(desc.color_blend_state.logic_op_enable)
                .logic_op(desc.color_blend_state.logic_op)
                .attachments(&color_blend_attachments)
                .blend_constants(desc.color_blend_state.blend_constants.map(|e| e.0));

        let pipeline_dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::default()
            .flags(desc.dynamic_state.flags)
            .dynamic_states(&desc.dynamic_state.dynamic_states);

        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::default()
            .view_mask(desc.rendering_state.view_mask)
            .color_attachment_formats(&desc.rendering_state.color_attachment_formats)
            .depth_attachment_format(desc.rendering_state.depth_attachment_format)
            .stencil_attachment_format(desc.rendering_state.stencil_attachment_format);

        let mut graphics_pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
            .push_next(&mut pipeline_rendering_create_info)
            .flags(desc.flags)
            .stages(&pipeline_shader_stage_create_infos)
            .input_assembly_state(&pipeline_input_assembly_state_create_info)
            .viewport_state(&pipeline_viewport_state_create_info)
            .rasterization_state(&pipeline_rasterization_state_create_info)
            .multisample_state(&pipeline_multisample_state_create_info)
            .color_blend_state(&pipeline_color_blend_state_create_info)
            .dynamic_state(&pipeline_dynamic_state_create_info)
            .layout(**pipeline_layout);

        if let Some(vertex_input_state_desc) = &desc.vertex_input_state {
            pipeline_vertex_input_state_create_info =
                vk::PipelineVertexInputStateCreateInfo::default()
                    .flags(vertex_input_state_desc.flags)
                    .vertex_binding_descriptions(unsafe {
                        tort_utils::slices::cast_unsafe(&vertex_input_state_desc.bindings)
                    })
                    .vertex_attribute_descriptions(unsafe {
                        tort_utils::slices::cast_unsafe(&vertex_input_state_desc.attributes)
                    });
            graphics_pipeline_create_info.p_vertex_input_state =
                &pipeline_vertex_input_state_create_info;
        }

        if let Some(depth_stencil_state_desc) = &desc.depth_stencil_state {
            pipeline_depth_stencil_state_create_info =
                vk::PipelineDepthStencilStateCreateInfo::default()
                    .flags(depth_stencil_state_desc.flags)
                    .depth_test_enable(depth_stencil_state_desc.depth_test_enable)
                    .depth_write_enable(depth_stencil_state_desc.depth_write_enable)
                    .depth_compare_op(depth_stencil_state_desc.depth_compare_op)
                    .depth_bounds_test_enable(depth_stencil_state_desc.depth_bounds_test_enable)
                    .stencil_test_enable(depth_stencil_state_desc.stencil_test_enable)
                    .front(
                        vk::StencilOpState::default()
                            .fail_op(depth_stencil_state_desc.front.fail_op)
                            .pass_op(depth_stencil_state_desc.front.pass_op)
                            .depth_fail_op(depth_stencil_state_desc.front.depth_fail_op)
                            .compare_op(depth_stencil_state_desc.front.compare_op)
                            .compare_mask(depth_stencil_state_desc.front.compare_mask)
                            .write_mask(depth_stencil_state_desc.front.write_mask)
                            .reference(depth_stencil_state_desc.front.reference),
                    )
                    .back(
                        vk::StencilOpState::default()
                            .fail_op(depth_stencil_state_desc.back.fail_op)
                            .pass_op(depth_stencil_state_desc.back.pass_op)
                            .depth_fail_op(depth_stencil_state_desc.back.depth_fail_op)
                            .compare_op(depth_stencil_state_desc.back.compare_op)
                            .compare_mask(depth_stencil_state_desc.back.compare_mask)
                            .write_mask(depth_stencil_state_desc.back.write_mask)
                            .reference(depth_stencil_state_desc.back.reference),
                    )
                    .min_depth_bounds(depth_stencil_state_desc.min_depth_bounds.0)
                    .max_depth_bounds(depth_stencil_state_desc.max_depth_bounds.0);

            graphics_pipeline_create_info.p_depth_stencil_state =
                &pipeline_depth_stencil_state_create_info;
        }

        let pipeline = unsafe {
            device.loader().create_graphics_pipelines(
                vk::PipelineCache::null(),
                slice::from_ref(&graphics_pipeline_create_info),
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
    pub fn desc(&self) -> &GraphicsPipelineDesc {
        &self.0.desc
    }

    #[inline]
    pub fn id(&self) -> &GraphicsPipelineId {
        &self.0.id
    }
}

impl Deref for GraphicsPipeline {
    type Target = vk::Pipeline;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.pipeline
    }
}

impl Pipeline for GraphicsPipeline {
    type Desc = GraphicsPipelineDesc;
    type Id = GraphicsPipelineId;
}
