use ash::Entry;
use ash::{vk, Instance};
use std::error::Error;
use std::ffi::CString;
use std::os::raw::c_char;

pub struct AppSettings<'a> {
    pub name: &'a str,
    pub layer_names: &'a [&'a str],
    pub vk_api_version: u32,
    pub extensions: Vec<*const c_char>,
}

pub struct App {
    pub entry: Entry,
    pub instance: Instance,
}

impl App {
    /// # Safety
    /// Creates an Entry instance; requires Vulkan support.
    pub unsafe fn new(entry: Entry, settings: AppSettings<'_>) -> Result<Self, Box<dyn Error>> {
        let app_name = CString::new(settings.name)?;

        let layer_names = settings
            .layer_names
            .iter()
            .map(|name| CString::new(*name).unwrap())
            .collect::<Vec<_>>();

        let layers_names_raw: Vec<_> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let enabled_extension_names = settings.extensions;
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

        Ok(Self { entry, instance })
    }

    /// # Safety
    /// Make sure all necessary resources have been cleaned up beforehand.
    pub unsafe fn destroy(&mut self) {
        self.instance.destroy_instance(None);
    }
}
