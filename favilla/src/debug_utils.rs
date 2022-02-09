use ash::extensions::ext::DebugUtils;
use ash::prelude::VkResult;
use ash::vk::{
    Bool32, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
    DebugUtilsMessengerCallbackDataEXT, DebugUtilsObjectNameInfoEXT, ObjectType,
};
use ash::{vk, Device, Entry, Instance};

use std::ffi::{c_void, CStr};

pub type DebugUtilsMessengerCallback = unsafe extern "system" fn(
    message_severity: DebugUtilsMessageSeverityFlagsEXT,
    message_types: DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const DebugUtilsMessengerCallbackDataEXT,
    p_user_data: *mut c_void,
) -> Bool32;

/// A debug utils helper. Only use this if the `DebugUtils` extension has been enabled.
pub struct DebugUtilsHelper {
    pub debug_utils: DebugUtils,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,
}

impl DebugUtilsHelper {
    /// Creates a new DebugUtilsHelper.
    /// Panics if creation fails.
    /// # Safety
    /// Requires support for DebugUtils.
    pub unsafe fn new(
        entry: &Entry,
        instance: &Instance,
        callback: DebugUtilsMessengerCallback,
    ) -> Self {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(callback));

        let debug_utils = DebugUtils::new(entry, instance);
        let debug_call_back = debug_utils
            .create_debug_utils_messenger(&debug_info, None)
            .unwrap();

        DebugUtilsHelper {
            debug_utils,
            debug_call_back,
        }
    }

    /// Set the name of an object.
    /// # Safety
    /// Requires support for DebugUtils.
    pub unsafe fn set_object_name(
        &self,
        device: &Device,
        object_handle: u64,
        object_type: ObjectType,
        name: &CStr,
    ) -> VkResult<()> {
        self.debug_utils.debug_utils_set_object_name(
            device.handle(),
            &DebugUtilsObjectNameInfoEXT::builder()
                .object_handle(object_handle)
                .object_type(object_type)
                .object_name(name)
                .build(),
        )
    }

    /// # Safety
    /// DebugUtils must be OK to destroy.
    pub unsafe fn destroy(&mut self) {
        self.debug_utils
            .destroy_debug_utils_messenger(self.debug_call_back, None);
    }
}
