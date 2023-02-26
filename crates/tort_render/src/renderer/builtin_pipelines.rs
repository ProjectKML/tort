use std::borrow::Cow;

use ash::vk;
use tort_asset::AssetServer;
use tort_ecs::{self as bevy_ecs, system::Resource};
use tort_utils::OrderedFloat;

use crate::backend::resource::pipeline::{
    ColorBlendStateDesc, DynamicStateDesc, GraphicsPipelineDesc, GraphicsPipelineId,
    InputAssemblyStateDesc, MultisampleStateDesc, PipelineCache, RasterizationStateDesc,
    RenderingStateDesc, ShaderStageDesc, ViewportStateDesc,
};

#[derive(Resource)]
pub struct BuiltinPipelines {
    pub geometry_pipeline: GraphicsPipelineId,
}

impl BuiltinPipelines {
    pub fn new(asset_server: &AssetServer, pipeline_cache: &mut PipelineCache) -> Self {
        let geometry_pipeline = pipeline_cache.queue_graphics_pipeline(&GraphicsPipelineDesc {
            stages: vec![
                ShaderStageDesc {
                    shader: asset_server.load("shaders/geometry_pass_mesh.spv"),
                    stage: vk::ShaderStageFlags::MESH_EXT,
                    entry_point: Cow::Borrowed("geometry::pass_mesh"),
                    ..Default::default()
                },
                ShaderStageDesc {
                    shader: asset_server.load("shaders/geometry_pass_frag.spv"),
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    entry_point: Cow::Borrowed("geometry::pass_frag"),
                    ..Default::default()
                },
            ],
            input_assembly_state: InputAssemblyStateDesc {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                ..Default::default()
            },
            viewport_state: ViewportStateDesc {
                viewports: vec![Default::default()],
                scissors: vec![Default::default()],
                ..Default::default()
            },
            rasterization_state: RasterizationStateDesc {
                polygon_mode: vk::PolygonMode::FILL,
                line_width: OrderedFloat(1.),
                ..Default::default()
            },
            multisample_state: MultisampleStateDesc {
                rasterization_samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            },
            color_blend_state: ColorBlendStateDesc {
                attachments: vec![Default::default()],
                ..Default::default()
            },
            dynamic_state: DynamicStateDesc {
                dynamic_states: vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR],
                ..Default::default()
            },
            rendering_state: RenderingStateDesc {
                color_attachment_formats: vec![],
                ..Default::default()
            },
            ..Default::default()
        });

        Self { geometry_pipeline }
    }
}
