use ash::vk;
use ash::vk::PhysicalDevice;

use ash::extensions::khr::Surface;

pub struct DeviceQueueFamilies {
    pub physical_device: PhysicalDevice,
    //TODO we might want to add support for multiple queue families in case no queue family
    // supports both graphics and presenting, but it seems no hardware actually works that way.
    pub queue_family_index: u32,
    pub surface_loader: Surface,
}

pub unsafe fn find(
    entry: &ash::Entry,
    instance: &ash::Instance,
    surface: vk::SurfaceKHR,
) -> DeviceQueueFamilies {
    let physical_devices = instance
        .enumerate_physical_devices()
        .expect("Physical device error");
    let surface_loader = Surface::new(entry, instance);

    let (physical_device, queue_family_index) = physical_devices
        .iter()
        .filter_map(|physical_device| {
            instance
                .get_physical_device_queue_family_properties(*physical_device)
                .iter()
                .enumerate()
                .filter_map(|(index, ref info)| {
                    let supports_graphic_and_surface =
                        info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                            && surface_loader
                                .get_physical_device_surface_support(
                                    *physical_device,
                                    index as u32,
                                    surface,
                                )
                                .expect("Get physical device surface support failed.");
                    if supports_graphic_and_surface {
                        Some((*physical_device, index))
                    } else {
                        None
                    }
                })
                .next()
        })
        .next()
        .expect("Couldn't find suitable device.");

    DeviceQueueFamilies {
        physical_device,
        queue_family_index: queue_family_index as u32,
        surface_loader,
    }
}
