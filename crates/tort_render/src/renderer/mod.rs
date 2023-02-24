mod frame_ctx;

use std::{env, mem, slice};

use anyhow::bail;
use ash::vk;
pub use frame_ctx::*;
use tort_ecs::system::{Res, ResMut};
use tort_utils::smallvec::{smallvec, SmallVec4, SmallVec8};

use crate::{
    backend::{Device, Instance, Swapchain},
    render_graph::{RenderGraph, RenderGraphCtx},
    view::{ExtractedWindows, WindowSurfaces},
};

pub fn init() -> (Instance, Device) {
    let instance = Instance::new(
        |layers| {
            if env::var("VALIDATION_LAYERS").is_ok() {
                layers.push_khronos_validation();
            }
        },
        |entry_loader, layers, extensions| {
            let version = entry_loader
                .try_enumerate_instance_version()?
                .unwrap_or(vk::API_VERSION_1_0);
            let major = vk::api_version_major(version);
            let minor = vk::api_version_minor(version);

            if major < 1 || minor < 3 {
                bail!(
                    "Only Vulkan {}.{}.{} is supported, but minimum supported version is 1.3",
                    major,
                    minor,
                    vk::api_version_patch(version)
                );
            }

            if layers.khronos_validation() {
                extensions.push_ext_debug_utils();
                extensions.push_ext_validation_features();
            }

            extensions.push_khr_get_surface_capabilities2();

            Ok(version)
        },
    )
    .unwrap();

    let physical_device = instance.find_optimal_physical_device();

    let device = unsafe {
        Device::new(
            instance.clone(),
            physical_device,
            |properties,
             _memory_properties,
             _queue_family_properties,
             extensions,
             _supported_features,
             enabled_features| {
                let version = properties.properties.api_version;
                let major = vk::api_version_minor(version);
                let minor = vk::api_version_minor(version);

                if major < 1 || minor < 1 {
                    bail!(
                        "Only Vulkan {}.{}.{} is supported, but minimum supported version is 1.3",
                        major,
                        minor,
                        vk::api_version_patch(version)
                    );
                }

                extensions.try_push_khr_portability_subset();
                extensions.push_ext_mesh_shader();
                extensions.push_khr_swapchain();

                enabled_features.features = vk::PhysicalDeviceFeatures::default();
                enabled_features.features_11 = vk::PhysicalDeviceVulkan11Features::default();
                enabled_features.features_12 =
                    vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
                enabled_features.features_13 = vk::PhysicalDeviceVulkan13Features::default();
                enabled_features.mesh_shader_features =
                    vk::PhysicalDeviceMeshShaderFeaturesEXT::default().mesh_shader(true);

                Ok(())
            },
        )
    }
    .unwrap();

    (instance, device)
}

unsafe fn update_render_graph(
    swapchain: &Swapchain,
    frame_ctx: &FrameCtx,
    render_graph: &RenderGraph,
) {
    let current_extent = &swapchain
        .surface_capabilities()
        .surface_capabilities
        .current_extent;
    let mut back_buffer_resources = swapchain
        .images()
        .iter()
        .map(|image| rps::vk_image_to_handle(*image))
        .collect::<SmallVec4<_>>();
    back_buffer_resources.rotate_right(frame_ctx.swapchain_image_shift);

    let back_buffer_desc = rps::ResourceDesc {
        type_: rps::ResourceType::IMAGE_2D,
        temporal_layers: swapchain.images().len() as _,
        buffer_image: rps::ResourceBufferImageDesc {
            image: rps::ResourceImageDesc {
                width: current_extent.width,
                height: current_extent.height,
                depth_or_array_layers: 1,
                mip_levels: 1,
                format: rps::format_from_vk(swapchain.used_surface_format().format),
                sample_count: 1,
            },
        },
        ..Default::default()
    };

    let args = [&back_buffer_desc as *const _ as *const _];
    let arg_resources = [back_buffer_resources.as_ptr()];

    let render_graph_update_info = rps::RenderGraphUpdateInfo {
        frame_index: frame_ctx.frame_index() as _,
        gpu_completed_frame_index: match frame_ctx.device_completed_frame_index() {
            Some(index) => index as _,
            None => rps::GPU_COMPLETED_FRAME_INDEX_NONE,
        },
        num_args: args.len() as _,
        args: args.as_ptr(),
        arg_resources: arg_resources.as_ptr(),
        ..Default::default()
    };

    rps::render_graph_update(**render_graph, &render_graph_update_info).unwrap();
}

pub fn render_system(
    windows: Res<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    _render_graph_ctx: Res<RenderGraphCtx>,
    render_graph: Res<RenderGraph>,
    mut frame_ctx: ResMut<FrameCtx>,
    instance: Res<Instance>,
    device: Res<Device>,
) {
    let frame = frame_ctx.current();

    let device_loader = device.loader();

    for window in windows.windows.values() {
        if window.physical_width == 0 || window.physical_height == 0 {
            continue
        }

        let (surface, swapchain) = window_surfaces.surfaces.get_mut(&window.entity).unwrap();

        unsafe {
            let fence = frame.fence();
            fence.wait_for(u64::MAX).unwrap();
            fence.reset().unwrap();

            update_render_graph(swapchain, &frame_ctx, &render_graph);

            let batch_layout = rps::render_graph_get_batch_layout(**render_graph).unwrap();

            for i in 0..batch_layout.num_cmd_batches {
                let batch = &*batch_layout.cmd_batches.offset(i as _);

                let queue_frame = frame.queue_frame(batch.queue_index);
                let command_buffer = Box::leak(Box::new(queue_frame.acquire_cmd_buffer())); //TODO: leaking here is awful

                device_loader
                    .begin_command_buffer(
                        **command_buffer,
                        &vk::CommandBufferBeginInfo::default()
                            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                    )
                    .unwrap();

                let render_graph_record_command_info = rps::RenderGraphRecordCommandInfo {
                    cmd_buffer: rps::vk_command_buffer_to_handle(**command_buffer),
                    frame_index: frame_ctx.frame_index() as _,
                    cmd_begin_index: batch.cmd_begin,
                    num_cmds: batch.num_cmds,
                    flags: rps::RecordCommandFlags::ENABLE_COMMAND_DEBUG_MARKERS,
                    ..Default::default()
                };

                rps::render_graph_record_commands(
                    **render_graph,
                    &render_graph_record_command_info,
                )
                .unwrap();

                device_loader.end_command_buffer(**command_buffer).unwrap();

                let timeline_semaphore = **queue_frame.timeline_semaphore();

                let (mut wait_semaphores, mut wait_semaphore_values, mut wait_dst_stage_masks) = (
                    (0..batch.num_wait_fences)
                        .map(|_| timeline_semaphore)
                        .collect::<SmallVec8<_>>(),
                    (0..batch.num_wait_fences)
                        .map(|i| {
                            *batch_layout
                                .wait_fence_indices
                                .offset((batch.wait_fences_begin + i) as _)
                                as _
                        })
                        .collect::<SmallVec8<_>>(),
                    (0..batch.num_wait_fences)
                        .map(|_| vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                        .collect::<SmallVec8<_>>(),
                );

                let (mut signal_semaphores, mut signal_semaphore_values): (
                    SmallVec4<_>,
                    SmallVec4<_>,
                ) = if batch.signal_fence_index == u32::MAX {
                    (smallvec![], smallvec![])
                } else {
                    (
                        smallvec![timeline_semaphore],
                        smallvec![batch.signal_fence_index as _],
                    )
                };

                let mut fence = vk::Fence::null();

                if i == 0 {
                    wait_semaphores.push(**frame.image_acquired_semaphore());
                    wait_semaphore_values.push(0);
                    wait_dst_stage_masks.push(vk::PipelineStageFlags::BOTTOM_OF_PIPE);
                }
                if i == batch_layout.num_cmd_batches - 1 {
                    signal_semaphores.push(**frame.rendering_done_semaphore());
                    signal_semaphore_values.push(0);
                    fence = **frame.fence();
                }

                let mut timeline_semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::default()
                    .wait_semaphore_values(&wait_semaphore_values)
                    .signal_semaphore_values(&signal_semaphore_values);

                let submit_info = vk::SubmitInfo::default()
                    .push_next(&mut timeline_semaphore_submit_info)
                    .wait_semaphores(&wait_semaphores)
                    .wait_dst_stage_mask(&wait_dst_stage_masks)
                    .command_buffers(slice::from_ref(command_buffer))
                    .signal_semaphores(&signal_semaphores);

                device_loader
                    .queue_submit(
                        **device.queue(batch.queue_index),
                        slice::from_ref(&submit_info),
                        fence,
                    )
                    .unwrap();
            }

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(slice::from_ref(frame.rendering_done_semaphore()))
                .swapchains(slice::from_ref(swapchain))
                .image_indices(slice::from_ref(&window.swap_chain_image_index));

            match device
                .swapchain_loader()
                .queue_present(**device.direct_queue(), &present_info)
            {
                Ok(is_suboptimal) => {
                    if is_suboptimal {
                        device.loader().device_wait_idle().unwrap();

                        let _ = mem::replace(
                            swapchain,
                            Swapchain::new(
                                instance.clone(),
                                surface.clone(),
                                device.clone(),
                                window.present_mode,
                                Some(swapchain),
                            )
                            .unwrap(),
                        );
                    }
                }
                Err(result) => {
                    if result != vk::Result::ERROR_OUT_OF_DATE_KHR {
                        panic!("vkQueuePresentKHR failed");
                    }
                }
            }
        }
    }

    frame_ctx.increment();
}
