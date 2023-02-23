use std::{ops::Deref, sync::Arc};

use anyhow::Result;
use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use tort_ecs::{self as bevy_ecs, system::Resource};

use crate::backend::Instance;

struct Inner {
    surface: vk::SurfaceKHR,
    instance: Instance,
}

impl Deref for Inner {
    type Target = vk::SurfaceKHR;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

impl Drop for Inner {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.instance
                .surface_loader()
                .destroy_surface(self.surface, None);
        }
    }
}

#[derive(Clone, Resource)]
pub struct Surface(Arc<Inner>);

impl Surface {
    pub fn new(
        instance: Instance,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self> {
        unsafe {
            let surface = ash_window::create_surface(
                instance.entry_loader(),
                instance.loader(),
                display_handle,
                window_handle,
                None,
            )?;
            Ok(Self(Arc::new(Inner { surface, instance })))
        }
    }

    #[inline]
    pub fn surface(&self) -> &vk::SurfaceKHR {
        &self.0.surface
    }
}
