use std::ffi::CString;

use ash::{vk, vk::Handle};

use crate::backend::{utils::BackendError, Device};

pub unsafe fn set_object_name<H: Handle>(
    device: &Device,
    handle: H,
    name: &str,
) -> Result<(), BackendError> {
    if device.instance().extensions().ext_debug_utils() {
        let object_name = CString::new(name)?;

        let debug_utils_object_name_info = vk::DebugUtilsObjectNameInfoEXT::default()
            .object_type(H::TYPE)
            .object_handle(handle.as_raw())
            .object_name(&object_name);

        device
            .instance()
            .debug_utils_loader()
            .set_debug_utils_object_name(device.loader().handle(), &debug_utils_object_name_info)?;
    }

    Ok(())
}
