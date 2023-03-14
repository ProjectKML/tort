use std::{
    collections::HashMap,
    mem,
    ops::{Deref, DerefMut},
};

use ash::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use tort_app::{App, IntoSystemAppConfig, Plugin};
use tort_ecs::{
    entity::Entity,
    event::EventReader,
    schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet},
    system::{NonSend, Query, Res, ResMut, Resource},
    {self as bevy_ecs},
};
use tort_utils::tracing::debug;
use tort_window::{
    CompositeAlphaMode, PresentMode, PrimaryWindow, RawHandleWrapper, Window, WindowClosed,
};

use crate::{
    backend::{Device, Instance, Surface, Swapchain},
    renderer::FrameCtx,
    Extract, ExtractSchedule, RenderApp, RenderSet,
};

/// Token to ensure a system runs on the main thread.
#[derive(Resource, Default)]
pub struct NonSendMarker;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WindowSystem {
    Prepare,
}

#[derive(Default)]
pub struct WindowRenderPlugin;

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedWindows>()
                .init_resource::<WindowSurfaces>()
                .init_non_send_resource::<NonSendMarker>()
                .add_system(extract_windows.in_schedule(ExtractSchedule))
                .configure_set(WindowSystem::Prepare.in_set(RenderSet::Prepare))
                .add_system(prepare_windows.in_set(WindowSystem::Prepare));
        }
    }
}

pub struct ExtractedWindow {
    /// An entity that contains the components in [`Window`].
    pub entity: Entity,
    pub handle: RawHandleWrapper,
    pub physical_width: u32,
    pub physical_height: u32,
    pub present_mode: PresentMode,
    pub swap_chain_image: vk::Image,
    pub swap_chain_image_view: vk::ImageView,
    pub swap_chain_image_index: u32,
    pub swap_chain_format: Option<vk::Format>,
    pub size_changed: bool,
    pub present_mode_changed: bool,
    pub alpha_mode: CompositeAlphaMode,
}

#[derive(Default, Resource)]
pub struct ExtractedWindows {
    pub primary: Option<Entity>,
    pub windows: HashMap<Entity, ExtractedWindow>,
}

impl Deref for ExtractedWindows {
    type Target = HashMap<Entity, ExtractedWindow>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.windows
    }
}

impl DerefMut for ExtractedWindows {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.windows
    }
}

fn extract_windows(
    mut extracted_windows: ResMut<ExtractedWindows>,
    mut closed: Extract<EventReader<WindowClosed>>,
    windows: Extract<Query<(Entity, &Window, &RawHandleWrapper, Option<&PrimaryWindow>)>>,
) {
    for (entity, window, handle, primary) in windows.iter() {
        if primary.is_some() {
            extracted_windows.primary = Some(entity);
        }

        let (new_width, new_height) = (
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        );

        let mut extracted_window = extracted_windows.entry(entity).or_insert(ExtractedWindow {
            entity,
            handle: handle.clone(),
            physical_width: new_width,
            physical_height: new_height,
            present_mode: window.present_mode,
            swap_chain_image: vk::Image::null(),
            swap_chain_image_view: vk::ImageView::null(),
            swap_chain_image_index: 0,
            swap_chain_format: None,
            size_changed: false,
            present_mode_changed: false,
            alpha_mode: window.composite_alpha_mode,
        });

        extracted_window.swap_chain_image = vk::Image::null();
        extracted_window.swap_chain_image_view = vk::ImageView::null();
        extracted_window.size_changed = new_width != extracted_window.physical_width
            || new_height != extracted_window.physical_height;
        extracted_window.present_mode_changed =
            window.present_mode != extracted_window.present_mode;

        if extracted_window.size_changed {
            debug!(
                "Window size changed from {}x{} to {}x{}",
                extracted_window.physical_width,
                extracted_window.physical_height,
                new_width,
                new_height
            );
            extracted_window.physical_width = new_width;
            extracted_window.physical_height = new_height;
        }

        if extracted_window.present_mode_changed {
            debug!(
                "Window Present Mode changed from {:?} to {:?}",
                extracted_window.present_mode, window.present_mode
            );
            extracted_window.present_mode = window.present_mode;
        }
    }

    for closed_window in closed.iter() {
        extracted_windows.remove(&closed_window.window);
    }
}

#[derive(Resource, Default)]
pub struct WindowSurfaces {
    pub surfaces: HashMap<Entity, (Surface, Swapchain)>,
}

fn prepare_windows(
    _marker: NonSend<NonSendMarker>,
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    instance: Res<Instance>,
    device: Res<Device>,
    mut frame_ctx: ResMut<FrameCtx>,
) {
    let frame = frame_ctx.current();

    let mut swapchain_image_shift = None;

    for window in windows.windows.values_mut() {
        let (surface, swapchain) = window_surfaces
            .surfaces
            .entry(window.entity)
            .or_insert_with(|| {
                let raw_handle = unsafe { window.handle.get_handle() };
                let surface = Surface::new(
                    instance.clone(),
                    raw_handle.raw_display_handle(),
                    raw_handle.raw_window_handle(),
                )
                .unwrap();

                (
                    surface.clone(),
                    Swapchain::new(
                        instance.clone(),
                        surface,
                        device.clone(),
                        window.present_mode,
                        None,
                    )
                    .unwrap(),
                )
            });

        if window.size_changed || window.present_mode_changed {
            unsafe { device.loader().device_wait_idle() }.unwrap();

            if window.physical_width == 0 || window.physical_height == 0 {
                continue
            }

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

            swapchain_image_shift = Some(frame_ctx.frame_index() % swapchain.images().len());

            debug!("Swapchain recreated");
        }

        if window.physical_width == 0 || window.physical_height == 0 {
            continue
        }

        let image_index = unsafe {
            match device.swapchain_loader().acquire_next_image(
                **swapchain,
                u64::MAX,
                **frame.image_acquired_semaphore(),
                vk::Fence::null(),
            ) {
                Ok((index, is_suboptimal)) => {
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

                        device
                            .swapchain_loader()
                            .acquire_next_image(
                                **swapchain,
                                u64::MAX,
                                **frame.image_acquired_semaphore(),
                                vk::Fence::null(),
                            )
                            .unwrap()
                            .0
                    } else {
                        index
                    }
                }
                Err(result) => {
                    if result != vk::Result::ERROR_OUT_OF_DATE_KHR {
                        panic!("vkAcquireNextImageKHR failed");
                    }

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

                    device
                        .swapchain_loader()
                        .acquire_next_image(
                            **swapchain,
                            u64::MAX,
                            **frame.image_acquired_semaphore(),
                            vk::Fence::null(),
                        )
                        .unwrap()
                        .0
                }
            }
        };

        window.swap_chain_image = swapchain.images()[image_index as usize];
        window.swap_chain_image_view = swapchain.image_views()[image_index as usize];
        window.swap_chain_image_index = image_index;
        window.swap_chain_format = Some(swapchain.used_surface_format().format);
    }

    if let Some(swapchain_image_shift) = swapchain_image_shift {
        frame_ctx.swapchain_image_shift = swapchain_image_shift;
    }
}
