use tort_ecs::{self as bevy_ecs, system::Resource};

use crate::backend::{
    command::{CommandBuffer, CommandBufferDesc, CommandPool, CommandPoolDesc},
    sync::{
        BinarySemaphore, BinarySemaphoreDesc, Fence, FenceDesc, TimelineSemaphore,
        TimelineSemaphoreDesc,
    },
    Device, Queue,
};

#[derive(Resource)]
pub struct FrameCtx {
    frames: Vec<Frame>,
    frame_offset: usize,
    frame_index: usize,
    pub swapchain_image_shift: usize,
    device: Device,
}

impl FrameCtx {
    pub fn new(device: Device, num_frames: usize) -> Self {
        let frames = (0..num_frames)
            .map(|_| Frame::new(device.clone()))
            .collect();

        Self {
            frames,
            frame_offset: 0,
            frame_index: 0,
            swapchain_image_shift: 0,
            device,
        }
    }

    #[inline]
    pub fn increment(&mut self) {
        self.frame_index += 1;
        self.frame_offset = self.frame_index % self.frames.len();
    }

    #[inline]
    pub fn current(&self) -> &Frame {
        &self.frames[self.frame_offset]
    }

    #[inline]
    pub fn current_mut(&mut self) -> &mut Frame {
        &mut self.frames[self.frame_offset]
    }

    #[inline]
    pub fn frame_index(&self) -> usize {
        self.frame_index
    }

    #[inline]
    pub fn device_completed_frame_index(&self) -> Option<usize> {
        if self.frame_index >= self.frames.len() {
            Some(self.frame_index - self.frames.len())
        } else {
            None
        }
    }
}

pub struct QueueFrame {
    timeline_semaphore: TimelineSemaphore,
    command_pool: CommandPool,
    device: Device,
}

impl QueueFrame {
    pub fn new(device: Device, queue: &Queue) -> Self {
        let timeline_semaphore =
            TimelineSemaphore::new(device.clone(), &TimelineSemaphoreDesc::default()).unwrap();

        let command_pool = CommandPool::new(
            device.clone(),
            &CommandPoolDesc {
                family_index: queue.family_index(),
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            timeline_semaphore,
            command_pool,
            device,
        }
    }

    #[inline]
    pub fn timeline_semaphore(&self) -> &TimelineSemaphore {
        &self.timeline_semaphore
    }

    pub fn acquire_cmd_buffer(&self) -> CommandBuffer {
        //TODO: reuse instead of recreate every time
        CommandBuffer::new(
            self.device.clone(),
            self.command_pool.clone(),
            &CommandBufferDesc::default(),
        )
        .unwrap()
    }
}

pub struct Frame {
    queue_frames: Vec<QueueFrame>,
    image_acquired_semaphore: BinarySemaphore,
    rendering_done_semaphore: BinarySemaphore,
    fence: Fence,
}

impl Frame {
    fn new(device: Device) -> Self {
        let queue_frames = device
            .queues()
            .iter()
            .map(|queue| QueueFrame::new(device.clone(), queue))
            .collect();
        let image_acquired_semaphore =
            BinarySemaphore::new(device.clone(), &BinarySemaphoreDesc::default()).unwrap();
        let rendering_done_semaphore =
            BinarySemaphore::new(device.clone(), &BinarySemaphoreDesc::default()).unwrap();
        let fence = Fence::new(
            device,
            &FenceDesc {
                signaled: true,
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            queue_frames,
            image_acquired_semaphore,
            rendering_done_semaphore,
            fence,
        }
    }

    #[inline]
    pub fn queue_frame(&self, queue_idx: u32) -> &QueueFrame {
        &self.queue_frames[queue_idx as usize]
    }

    #[inline]
    pub fn image_acquired_semaphore(&self) -> &BinarySemaphore {
        &self.image_acquired_semaphore
    }

    #[inline]
    pub fn rendering_done_semaphore(&self) -> &BinarySemaphore {
        &self.rendering_done_semaphore
    }

    #[inline]
    pub fn fence(&self) -> &Fence {
        &self.fence
    }
}
