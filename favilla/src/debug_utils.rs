use ash::ext::debug_utils::Instance as DebugInstance;
use ash::vk::DebugUtilsMessengerCallbackDataEXT;
use ash::vk::{Bool32, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT};
use ash::{vk, Entry, Instance};

use std::ffi::c_void;

pub type DebugUtilsMessengerCallback = unsafe extern "system" fn(
    message_severity: DebugUtilsMessageSeverityFlagsEXT,
    message_types: DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const DebugUtilsMessengerCallbackDataEXT<'_>,
    p_user_data: *mut c_void,
) -> Bool32;

/// A debug utils helper. Only use this if the `DebugUtils` extension has been enabled.
pub struct DebugUtilsInstanceHelper {
    pub debug_instance: DebugInstance,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,
}

impl DebugUtilsInstanceHelper {
    /// Creates a new DebugUtilsHelper.
    /// Panics if creation fails.
    /// # Safety
    /// Requires support for DebugUtils.
    pub unsafe fn new(
        entry: &Entry,
        instance: &Instance,
        callback: DebugUtilsMessengerCallback,
    ) -> Self {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(callback));

        let debug_instance = DebugInstance::new(entry, instance);

        let debug_call_back = debug_instance
            .create_debug_utils_messenger(&debug_info, None)
            .unwrap();

        DebugUtilsInstanceHelper {
            debug_instance,
            debug_call_back,
        }
    }

    /// # Safety
    /// DebugUtils must be OK to destroy.
    pub unsafe fn destroy(&mut self) {
        self.debug_instance
            .destroy_debug_utils_messenger(self.debug_call_back, None);
    }
}
