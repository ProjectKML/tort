mod builtin_pipelines;
mod frame_ctx;

use std::{env, mem, slice};

use anyhow::bail;
use ash::vk;
pub use builtin_pipelines::*;
pub use frame_ctx::*;
use tort_ecs::system::{Res, ResMut};

use crate::{
    backend::{resource::pipeline::PipelineCache, Device, Instance, Swapchain},
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
                enabled_features.features_13 = vk::PhysicalDeviceVulkan13Features::default()
                    .dynamic_rendering(true)
                    .synchronization2(true);
                enabled_features.mesh_shader_features =
                    vk::PhysicalDeviceMeshShaderFeaturesEXT::default().mesh_shader(true);

                Ok(())
            },
        )
    }
    .unwrap();

    (instance, device)
}

pub fn render_system(
    windows: Res<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    frame_ctx: ResMut<FrameCtx>,
    instance: Res<Instance>,
    device: Res<Device>,
    pipeline_cache: Res<PipelineCache>,
    builtin_pipelines: Res<BuiltinPipelines>,
) {
    let frame = frame_ctx.current();

    let device_loader = device.loader();

    let queue_frame = frame.queue_frame(0);
    let command_pool = **queue_frame.command_pool();
    let command_buffer = **queue_frame.command_buffer();

    let image_acquired_semaphore = frame.image_acquired_semaphore();
    let rendering_done_semaphore = frame.rendering_done_semaphore();

    for window in windows.windows.values() {
        if window.physical_width == 0 || window.physical_height == 0 {
            continue
        }

        let (surface, swapchain) = window_surfaces.surfaces.get_mut(&window.entity).unwrap();

        unsafe {
            let fence = frame.fence();
            fence.wait_for(u64::MAX).unwrap();
            fence.reset().unwrap();

            device_loader
                .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                .unwrap();

            device_loader
                .begin_command_buffer(
                    command_buffer,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();

            device_loader.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfo::default().image_memory_barriers(slice::from_ref(
                    &vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image(window.swap_chain_image)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .level_count(1)
                                .layer_count(1),
                        ),
                )),
            );

            let color_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(window.swap_chain_image_view)
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [100.0 / 255.0, 149.0 / 255.0, 237.0 / 255.0, 1.0],
                    },
                });

            let rendering_info = vk::RenderingInfo::default()
                .render_area(
                    vk::Rect2D::default().extent(
                        vk::Extent2D::default()
                            .width(window.physical_width)
                            .height(window.physical_height),
                    ),
                )
                .layer_count(1)
                .color_attachments(slice::from_ref(&color_attachment));

            device_loader.cmd_begin_rendering(command_buffer, &rendering_info);

            if let Some(pipeline) =
                pipeline_cache.get_graphics_pipeline(&builtin_pipelines.geometry_pipeline)
            {
                device_loader.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    **pipeline,
                );

                device_loader.cmd_set_viewport(
                    command_buffer,
                    0,
                    slice::from_ref(
                        &vk::Viewport::default()
                            .width(1600.0)
                            .height(900.0)
                            .max_depth(1.0),
                    ),
                );
                device_loader.cmd_set_scissor(
                    command_buffer,
                    0,
                    slice::from_ref(&vk::Rect2D::default().extent(vk::Extent2D {
                        width: 1600,
                        height: 900,
                    })),
                );

                device
                    .mesh_shader_loader()
                    .cmd_draw_mesh_tasks(command_buffer, 1, 1, 1);
            }

            device_loader.cmd_end_rendering(command_buffer);

            device_loader.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfo::default().image_memory_barriers(slice::from_ref(
                    &vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
                        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .image(window.swap_chain_image)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .level_count(1)
                                .layer_count(1),
                        ),
                )),
            );

            device_loader.end_command_buffer(command_buffer).unwrap();

            let direct_queue = **device.direct_queue();

            device_loader
                .queue_submit(
                    direct_queue,
                    slice::from_ref(
                        &vk::SubmitInfo::default()
                            .wait_semaphores(slice::from_ref(image_acquired_semaphore))
                            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                            .command_buffers(slice::from_ref(&command_buffer))
                            .signal_semaphores(slice::from_ref(rendering_done_semaphore)),
                    ),
                    **fence,
                )
                .unwrap();

            match device.swapchain_loader().queue_present(
                direct_queue,
                &vk::PresentInfoKHR::default()
                    .wait_semaphores(slice::from_ref(rendering_done_semaphore))
                    .swapchains(slice::from_ref(swapchain))
                    .image_indices(slice::from_ref(&window.swap_chain_image_index)),
            ) {
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
}
