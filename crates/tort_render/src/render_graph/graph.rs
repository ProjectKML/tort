use std::ops::Deref;

use ash::vk;
use rps::{declare_rpsl_entry, entry_ref, RpsResult};
use tort_ecs::{self as bevy_ecs, system::Resource};
use tort_utils::smallvec::SmallVec4;

use crate::render_graph::RenderGraphCtx;

fn rps_queue_flags_from_vk(flags: vk::QueueFlags) -> rps::QueueFlags {
    let mut result = rps::QueueFlags::NONE;
    if (flags & vk::QueueFlags::GRAPHICS) == vk::QueueFlags::GRAPHICS {
        result |= rps::QueueFlags::GRAPHICS;
    }
    if (flags & vk::QueueFlags::COMPUTE) == vk::QueueFlags::COMPUTE {
        result |= rps::QueueFlags::COMPUTE;
    }
    if (flags & vk::QueueFlags::TRANSFER) == vk::QueueFlags::TRANSFER {
        result |= rps::QueueFlags::COPY;
    }
    result
}

declare_rpsl_entry!(basic, main);

#[derive(Resource)]
pub struct RenderGraph {
    render_graph: rps::RenderGraph,
}

impl RenderGraph {
    pub fn new(render_graph_ctx: &RenderGraphCtx) -> RpsResult<Self> {
        let device = render_graph_ctx.device();

        let queue_family_properties = &device.queue_family_properties().queue_family_properties;
        let queue_infos = device
            .queues()
            .iter()
            .map(|queue| {
                rps_queue_flags_from_vk(
                    queue_family_properties[queue.family_index() as usize].queue_flags,
                )
            })
            .collect::<SmallVec4<_>>();

        let render_graph_create_info = rps::RenderGraphCreateInfo {
            schedule_info: rps::RenderGraphCreateScheduleInfo {
                schedule_flags: rps::ScheduleFlags::DEFAULT,
                num_queues: queue_infos.len() as _,
                queue_infos: queue_infos.as_ptr(),
            },
            main_entry_create_info: rps::ProgramCreateInfo {
                rpsl_entry_point: unsafe { entry_ref!(basic, main) },
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            let render_graph = rps::render_graph_create(
                *render_graph_ctx.rps_device(),
                &render_graph_create_info,
            )?;
            let main_entry = rps::render_graph_get_main_entry(render_graph);

            unsafe extern "C" fn swapchain_pass_cb(_context: *const rps::CmdCallbackContext) {}

            let cmd_callback = rps::CmdCallback {
                pfn_callback: Some(swapchain_pass_cb),
                ..Default::default()
            };
            rps::program_bind_node_callback(
                main_entry,
                b"SwapchainPass\0".as_ptr().cast(),
                &cmd_callback,
            )
            .unwrap();

            Ok(Self { render_graph })
        }
    }
}

impl Deref for RenderGraph {
    type Target = rps::RenderGraph;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.render_graph
    }
}

impl Drop for RenderGraph {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            rps::render_graph_destroy(self.render_graph);
        }
    }
}
