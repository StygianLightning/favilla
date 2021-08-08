use ash::extensions::ext::DebugUtils;
use ash::Entry;
use ash::{vk, Instance};
use std::error::Error;
use std::ffi::CString;

pub struct AppSettings<'a> {
    pub name: &'a str,
    pub layer_names: &'a [&'a str],
    pub add_debug_utils: bool,
    pub vk_api_version: u32,
    pub extensions: Vec<CString>,
}

pub struct DebugUtilsHelper {
    pub debug_utils: DebugUtils,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,
}

pub struct App {
    pub entry: Entry,
    pub debug_utils_helper: Option<DebugUtilsHelper>,
    pub instance: Instance,
}

impl App {
    pub unsafe fn new(settings: AppSettings<'_>) -> Result<Self, Box<dyn Error>> {
        let entry = Entry::new()?;

        let app_name = CString::new(settings.name)?;

        let layer_names = settings
            .layer_names
            .iter()
            .map(|name| CString::new(*name).unwrap())
            .collect::<Vec<_>>();

        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let mut extensions = settings.extensions;

        if settings.add_debug_utils {
            extensions.push(DebugUtils::name().to_owned());
        }

        let enabled_extension_names = extensions.iter().map(|e| e.as_ptr()).collect::<Vec<_>>();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(settings.vk_api_version);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&enabled_extension_names);

        let instance: Instance = entry.create_instance(&create_info, None)?;

        let debug_utils_helper = if settings.add_debug_utils {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                .pfn_user_callback(Some(crate::debug_callback::vulkan_debug_callback));

            let debug_utils = DebugUtils::new(&entry, &instance);
            let debug_call_back = debug_utils
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();
            Some(DebugUtilsHelper {
                debug_utils,
                debug_call_back,
            })
        } else {
            None
        };

        Ok(Self {
            entry,
            debug_utils_helper,
            instance,
        })
    }

    pub unsafe fn destroy(&mut self) {
        if let Some(DebugUtilsHelper {
            debug_call_back,
            debug_utils,
        }) = &self.debug_utils_helper.take()
        {
            debug_utils.destroy_debug_utils_messenger(*debug_call_back, None);
        }
        self.instance.destroy_instance(None);
    }
}
