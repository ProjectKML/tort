use std::{
    env,
    ffi::CStr,
    os::raw::{c_char, c_void},
    sync::Arc,
};

use anyhow::Result;
use ash::{
    extensions::{
        ext,
        ext::DebugUtils,
        khr,
        khr::{GetSurfaceCapabilities2, Surface},
    },
    prelude::VkResult,
    vk, Entry,
};
use log::log;
use tort_ecs::{self as bevy_ecs, system::Resource};

use crate::backend::utils::message_severity;

#[inline]
fn application_info_from_cargo_toml(api_version: u32) -> vk::ApplicationInfo<'static> {
    let version = vk::make_api_version(
        0,
        env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
        env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
    );

    let application_name = concat!(env!("CARGO_PKG_NAME"), "_game\0");
    let engine_name = concat!(env!("CARGO_PKG_NAME"), "\0");

    unsafe {
        vk::ApplicationInfo::default()
            .application_name(CStr::from_bytes_with_nul_unchecked(
                application_name.as_bytes(),
            ))
            .application_version(version)
            .engine_name(CStr::from_bytes_with_nul_unchecked(engine_name.as_bytes()))
            .engine_version(version)
            .api_version(api_version)
    }
}

pub struct InstanceLayers {
    supported: Vec<vk::LayerProperties>,
    enabled: Vec<*const c_char>,

    khronos_validation: bool,
}

impl InstanceLayers {
    pub fn new(entry_loader: &Entry) -> VkResult<Self> {
        let supported = entry_loader.enumerate_instance_layer_properties()?;

        Ok(Self {
            supported,
            enabled: Vec::new(),

            khronos_validation: false,
        })
    }

    #[inline]
    unsafe fn try_push(&mut self, name: *const c_char) -> bool {
        if self
            .supported
            .iter()
            .any(|e| libc::strcmp(e.layer_name.as_ptr(), name) == 0)
        {
            self.enabled.push(name);
            true
        } else {
            false
        }
    }

    pub fn try_push_khronos_validation(&mut self) -> bool {
        if unsafe { self.try_push(b"VK_LAYER_KHRONOS_validation\0".as_ptr().cast()) } {
            self.khronos_validation = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khronos_validation(&mut self) {
        assert!(self.try_push_khronos_validation());
    }

    #[inline]
    pub fn khronos_validation(&self) -> bool {
        self.khronos_validation
    }
}

unsafe impl Send for InstanceLayers {}
unsafe impl Sync for InstanceLayers {}

pub struct InstanceExtensions {
    supported: Vec<vk::ExtensionProperties>,
    supported_khronos_validation: Vec<vk::ExtensionProperties>,
    enabled: Vec<*const c_char>,

    ext_debug_utils: bool,
    ext_validation_features: bool,
    khr_get_surface_capabilities2: bool,
    khr_portability_enumeration: bool,
    khr_surface: bool,
}

impl InstanceExtensions {
    pub fn new(entry_loader: &Entry, layers: &InstanceLayers) -> VkResult<Self> {
        let supported = entry_loader.enumerate_instance_extension_properties(None)?;

        let supported_khronos_validation = if layers.khronos_validation() {
            entry_loader.enumerate_instance_extension_properties(Some(unsafe {
                CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")
            }))?
        } else {
            Vec::new()
        };

        Ok(Self {
            supported,
            supported_khronos_validation,
            enabled: Vec::new(),

            ext_debug_utils: false,
            ext_validation_features: false,
            khr_get_surface_capabilities2: false,
            khr_portability_enumeration: false,
            khr_surface: false,
        })
    }

    #[inline]
    unsafe fn try_push(&mut self, name: *const c_char) -> bool {
        if self
            .supported
            .iter()
            .any(|e| libc::strcmp(e.extension_name.as_ptr(), name) == 0)
            || self
                .supported_khronos_validation
                .iter()
                .any(|e| libc::strcmp(e.extension_name.as_ptr(), name) == 0)
        {
            self.enabled.push(name);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn try_push_ext_debug_utils(&mut self) -> bool {
        if unsafe { self.try_push(DebugUtils::name().as_ptr()) } {
            self.ext_debug_utils = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_ext_debug_utils(&mut self) {
        assert!(self.try_push_ext_debug_utils())
    }

    #[inline]
    pub fn ext_debug_utils(&self) -> bool {
        self.ext_debug_utils
    }

    #[inline]
    pub fn try_push_ext_validation_features(&mut self) -> bool {
        if unsafe { self.try_push(b"VK_EXT_validation_features\0".as_ptr().cast()) } {
            self.ext_validation_features = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_ext_validation_features(&mut self) {
        assert!(self.try_push_ext_validation_features())
    }

    #[inline]
    pub fn ext_validation_features(&self) -> bool {
        self.ext_validation_features
    }

    #[inline]
    pub fn try_push_khr_get_surface_capabilities2(&mut self) -> bool {
        if unsafe { self.try_push(GetSurfaceCapabilities2::name().as_ptr()) } {
            self.khr_get_surface_capabilities2 = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khr_get_surface_capabilities2(&mut self) {
        assert!(self.try_push_khr_get_surface_capabilities2())
    }

    #[inline]
    pub fn khr_get_surface_capabilities2(&self) -> bool {
        self.khr_get_surface_capabilities2
    }

    #[inline]
    pub fn try_push_khr_portability_enumeration(&mut self) -> bool {
        if unsafe { self.try_push(b"VK_KHR_portability_enumeration\0".as_ptr().cast()) } {
            self.khr_portability_enumeration = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khr_portability_enumeration(&mut self) {
        assert!(self.try_push_khr_portability_enumeration());
    }

    #[inline]
    pub fn khr_portability_enumeration(&self) -> bool {
        self.khr_portability_enumeration
    }

    #[inline]
    pub fn try_push_khr_surface(&mut self) -> bool {
        if unsafe { self.try_push(Surface::name().as_ptr()) } {
            self.khr_surface = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khr_surface(&mut self) {
        assert!(self.try_push_khr_surface())
    }

    #[inline]
    pub fn khr_surface(&self) -> bool {
        self.khr_surface
    }
}

unsafe impl Send for InstanceExtensions {}
unsafe impl Sync for InstanceExtensions {}

struct Inner {
    entry_loader: Entry,

    loader: ash::Instance,
    debug_utils_loader: DebugUtils,
    get_surface_capabilities2_loader: GetSurfaceCapabilities2,
    surface_loader: Surface,

    layers: InstanceLayers,
    extensions: InstanceExtensions,

    debug_utils_messenger: vk::DebugUtilsMessengerEXT,

    physical_devices: Vec<vk::PhysicalDevice>,
}

impl Drop for Inner {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if self.debug_utils_messenger != vk::DebugUtilsMessengerEXT::null() {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            }

            self.loader.destroy_instance(None);
        }
    }
}

#[derive(Clone, Resource)]
pub struct Instance(Arc<Inner>);

impl Instance {
    pub fn new(
        layer_callback: impl FnOnce(&mut InstanceLayers),
        callback: impl FnOnce(&Entry, &InstanceLayers, &mut InstanceExtensions) -> Result<u32>,
    ) -> Result<Self> {
        unsafe {
            let entry_loader = {
                if let Ok(path) = env::var("TORT_VULKAN_LIB") {
                    Entry::load_from(path)
                } else {
                    Entry::load()
                }
            }?;

            //Layers
            let mut layers = InstanceLayers::new(&entry_loader)?;
            layer_callback(&mut layers);

            let mut extensions = InstanceExtensions::new(&entry_loader, &layers)?;

            if cfg!(all(
                unix,
                not(target_os = "android"),
                not(target_os = "macos")
            )) {
                extensions.try_push(khr::XlibSurface::name().as_ptr());
                extensions.try_push(khr::XcbSurface::name().as_ptr());
                extensions.try_push(khr::WaylandSurface::name().as_ptr());
            }
            if cfg!(target_os = "android") {
                extensions.try_push(khr::AndroidSurface::name().as_ptr());
            }
            if cfg!(target_os = "windows") {
                extensions.try_push(khr::Win32Surface::name().as_ptr());
            }
            if cfg!(target_os = "macos") {
                extensions.try_push(ext::MetalSurface::name().as_ptr());
            }

            extensions.push_khr_surface();

            let application_info = application_info_from_cargo_toml(callback(
                &entry_loader,
                &layers,
                &mut extensions,
            )?);

            let enabled_validation_features = [
                //vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
                vk::ValidationFeatureEnableEXT::DEBUG_PRINTF,
                vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
            ];
            let disabled_validation_features = [];
            let mut validation_features = vk::ValidationFeaturesEXT::default()
                .enabled_validation_features(&enabled_validation_features)
                .disabled_validation_features(&disabled_validation_features);

            let mut instance_create_info = vk::InstanceCreateInfo::default()
                .application_info(&application_info)
                .enabled_extension_names(&extensions.enabled)
                .enabled_layer_names(&layers.enabled);

            if extensions.khr_portability_enumeration() {
                instance_create_info.flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
            }

            if extensions.ext_validation_features() {
                instance_create_info = instance_create_info.push_next(&mut validation_features);
            }

            let loader = entry_loader.create_instance(&instance_create_info, None)?;
            let debug_utils_loader = DebugUtils::new(&entry_loader, &loader);
            let get_surface_capabilities2_loader =
                GetSurfaceCapabilities2::new(&entry_loader, &loader);
            let surface_loader = Surface::new(&entry_loader, &loader);

            let debug_utils_messenger = if extensions.ext_debug_utils() {
                let debug_utils_messenger_create_info =
                    vk::DebugUtilsMessengerCreateInfoEXT::default()
                        .message_severity(
                            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                        )
                        .message_type(
                            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                        )
                        .pfn_user_callback(Some(debug_callback));

                debug_utils_loader
                    .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)?
            } else {
                vk::DebugUtilsMessengerEXT::null()
            };

            let physical_devices = loader.enumerate_physical_devices()?;

            Ok(Self(Arc::new(Inner {
                entry_loader,

                loader,
                debug_utils_loader,
                get_surface_capabilities2_loader,
                surface_loader,

                layers,
                extensions,

                debug_utils_messenger,

                physical_devices,
            })))
        }
    }

    pub fn find_optimal_physical_device(&self) -> vk::PhysicalDevice {
        let mut heap_size: u64 = 0;
        let mut physical_device = vk::PhysicalDevice::null();

        for current_physical_device in self.0.physical_devices.iter() {
            let properties = unsafe {
                self.0
                    .loader
                    .get_physical_device_properties(*current_physical_device)
            };

            if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
                continue
            }

            let memory_properties = unsafe {
                self.0
                    .loader
                    .get_physical_device_memory_properties(*current_physical_device)
            };
            let mut current_heap_size: u64 = 0;

            for i in 0..memory_properties.memory_heap_count as usize {
                let current_heap = &memory_properties.memory_heaps[i];

                if (current_heap.flags & vk::MemoryHeapFlags::DEVICE_LOCAL)
                    == vk::MemoryHeapFlags::DEVICE_LOCAL
                {
                    current_heap_size += current_heap.size;
                }
            }

            if current_heap_size > heap_size {
                heap_size = current_heap_size;
                physical_device = *current_physical_device;
            }
        }

        if physical_device == vk::PhysicalDevice::null() {
            physical_device = self.0.physical_devices[0];
        }

        physical_device
    }

    #[inline]
    pub fn entry_loader(&self) -> &Entry {
        &self.0.entry_loader
    }

    #[inline]
    pub fn loader(&self) -> &ash::Instance {
        &self.0.loader
    }

    #[inline]
    pub fn debug_utils_loader(&self) -> &DebugUtils {
        &self.0.debug_utils_loader
    }

    #[inline]
    pub fn get_surface_capabilities2_loader(&self) -> &GetSurfaceCapabilities2 {
        &self.0.get_surface_capabilities2_loader
    }

    #[inline]
    pub fn surface_loader(&self) -> &Surface {
        &self.0.surface_loader
    }

    #[inline]
    pub fn layers(&self) -> &InstanceLayers {
        &self.0.layers
    }

    #[inline]
    pub fn extensions(&self) -> &InstanceExtensions {
        &self.0.extensions
    }
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    log!(
        message_severity::to_log_level(message_severity),
        "[{:?}] {}",
        message_types,
        CStr::from_ptr((*callback_data).p_message).to_str().unwrap()
    );

    vk::FALSE
}
