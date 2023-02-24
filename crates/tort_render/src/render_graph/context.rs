use std::{
    ffi::{c_char, c_void},
    mem,
};

use ash::{vk, vk::Handle};
use rps::RpsResult;
use tort_core::allocator;
use tort_ecs::{self as bevy_ecs, system::Resource};
use tort_utils::tracing::info;

use crate::backend::Device;

unsafe extern "C" fn alloc(
    _user_context: *mut c_void,
    size: usize,
    alignment: usize,
) -> *mut c_void {
    allocator::allocate_aligned(size, alignment)
}

unsafe extern "C" fn realloc(
    _user_context: *mut c_void,
    old_buffer: *mut c_void,
    _old_size: usize,
    new_size: usize,
    alignment: usize,
) -> *mut c_void {
    allocator::reallocate_aligned(old_buffer, new_size, alignment)
}

unsafe extern "C" fn free(_user_context: *mut c_void, buffer: *mut c_void) {
    allocator::deallocate_aligned(buffer)
}

unsafe extern "C" fn printf(_user_context: *mut c_void, format: *const c_char, mut args: ...) {
    let mut buffer = String::new();
    printf_compat::format(
        format,
        args.as_va_list(),
        printf_compat::output::fmt_write(&mut buffer),
    );
    info!("{buffer}");
}

unsafe extern "C" fn vprintf(_user_context: *mut c_void, format: *const c_char, vl: rps::VaList) {
    let mut buffer = String::new();
    printf_compat::format(
        format,
        mem::transmute(vl),
        printf_compat::output::fmt_write(&mut buffer),
    );
    info!("{buffer}");
}

unsafe extern "C" fn record_debug_marker(
    user_context: *mut c_void,
    args: *const rps::RuntimeOpRecordDebugMarkerArgs,
) {
    let args = &*args;

    let device: &Device = &*user_context.cast();
    let debug_utils_loader = device.instance().debug_utils_loader();

    if device.instance().extensions().ext_debug_utils() {
        let command_buffer = rps::vk_command_buffer_from_handle(args.command_buffer);

        match args.mode {
            rps::RuntimeDebugMarkerMode::BEGIN => {
                debug_utils_loader.cmd_begin_debug_utils_label(
                    command_buffer,
                    &vk::DebugUtilsLabelEXT {
                        p_label_name: args.text,
                        color: [0.0, 1.0, 0.0, 1.0],
                        ..Default::default()
                    },
                )
            }
            rps::RuntimeDebugMarkerMode::LABEL => {
                debug_utils_loader.cmd_insert_debug_utils_label(
                    command_buffer,
                    &vk::DebugUtilsLabelEXT {
                        p_label_name: args.text,
                        color: [0.0, 1.0, 0.0, 1.0],
                        ..Default::default()
                    },
                )
            }
            rps::RuntimeDebugMarkerMode::END => {
                debug_utils_loader.cmd_end_debug_utils_label(command_buffer)
            }
            _ => panic!("Unknown rps::RuntimeDebugMarkerMode: {:?}", args.mode),
        }
    }
}

unsafe extern "C" fn set_debug_name(
    user_context: *mut c_void,
    args: *const rps::RuntimeOpSetDebugNameArgs,
) {
    let args = &*args;

    let device: &Device = &*user_context.cast();

    if device.instance().extensions().ext_debug_utils() {
        let debug_utils_object_name_info = vk::DebugUtilsObjectNameInfoEXT {
            object_type: match args.resource_type {
                rps::ResourceType::BUFFER => vk::ObjectType::BUFFER,
                rps::ResourceType::IMAGE_1D
                | rps::ResourceType::IMAGE_2D
                | rps::ResourceType::IMAGE_3D => vk::ObjectType::IMAGE,
                _ => panic!("Unknown rps::ResourceType: {:?}", args.resource_type),
            },
            object_handle: rps::vk_image_from_handle(args.resource).as_raw(),
            p_object_name: args.name,
            ..Default::default()
        };

        device
            .instance()
            .debug_utils_loader()
            .set_debug_utils_object_name(**device, &debug_utils_object_name_info)
            .unwrap();
    }
}

#[derive(Resource)]
pub struct RenderGraphCtx {
    rps_device: rps::Device,
    device: Device,
}

impl RenderGraphCtx {
    pub fn new(device: Device) -> RpsResult<Self> {
        let device_create_info = rps::DeviceCreateInfo {
            allocator: rps::Allocator {
                pfn_alloc: Some(alloc),
                pfn_realloc: Some(realloc),
                pfn_free: Some(free),
                ..Default::default()
            },
            printer: rps::Printer {
                pfn_printf: Some(printf),
                pfn_vprintf: Some(vprintf),
                ..Default::default()
            },
            ..Default::default()
        };

        let runtime_create_info = rps::RuntimeDeviceCreateInfo {
            user_context: Box::leak(Box::new(device.clone())) as *mut _ as *mut _,
            callbacks: rps::RuntimeCallbacks {
                pfn_record_debug_marker: Some(record_debug_marker),
                pfn_set_debug_name: Some(set_debug_name),
                ..Default::default()
            },
        };

        let vulkan_functions = rps::VKFunctions::new(device.instance().loader(), device.loader());

        let runtime_device_create_info = rps::VKRuntimeDeviceCreateInfo {
            device_create_info: &device_create_info,
            runtime_create_info: &runtime_create_info,
            vulkan_functions: &vulkan_functions,
            vk_device: *device,
            vk_physical_device: *device.physical_device(),
            flags: rps::VKRuntimeFlags::DONT_FLIP_VIEWPORT,
        };

        let rps_device = unsafe { rps::vk_runtime_device_create(&runtime_device_create_info) }?;

        Ok(Self { rps_device, device })
    }

    #[inline]
    pub fn rps_device(&self) -> &rps::Device {
        &self.rps_device
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }
}

impl Drop for RenderGraphCtx {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            rps::device_destroy(self.rps_device);
        }
    }
}
