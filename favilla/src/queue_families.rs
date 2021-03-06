use ash::extensions::khr::Surface;
use ash::vk;
use ash::vk::{PhysicalDevice, PhysicalDeviceProperties, PhysicalDeviceType};
use std::os::raw::c_char;
use tracing::info;

/// A struct holding a physical device, a queue family index and a surface.
pub struct DeviceQueueFamilies {
    pub physical_device: vk::PhysicalDevice,
    //We might want to add support for multiple queue families in case no queue family
    // supports both graphics and presenting, but it seems no hardware actually works that way currently,
    // so we're sticking to the simpler API for now.
    pub queue_family_index: u32,
    pub surface_loader: Surface,
}

/// Function with a default implementation to get a suitable queue family;
/// selects the first discrete gpu unless an index is specified,
/// in which case the candidate device with the given index is used.
/// Devices without support for graphics or the given surface are ignored.
/// # Safety
/// The given instance must be valid and compatible with the given surface.
pub unsafe fn select(
    entry: &ash::Entry,
    instance: &ash::Instance,
    surface: vk::SurfaceKHR,
    index: Option<usize>,
) -> DeviceQueueFamilies {
    let physical_devices = instance
        .enumerate_physical_devices()
        .expect("Physical device error");
    let surface_loader = Surface::new(entry, instance);

    struct Candidate {
        physical_device: PhysicalDevice,
        queue_family_index: usize,
        physical_device_properties: PhysicalDeviceProperties,
    }

    // Select device with given index that supports graphics;
    // default to the first device with the highest queue count.
    let devices_and_queues = physical_devices
        .iter()
        .filter_map(|physical_device| {
            instance
                .get_physical_device_queue_family_properties(*physical_device)
                .iter()
                .enumerate()
                .find_map(|(queue_family_index, ref info)| {
                    let supports_graphic_and_surface =
                        info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                            && surface_loader
                                .get_physical_device_surface_support(
                                    *physical_device,
                                    queue_family_index as u32,
                                    surface,
                                )
                                .expect("Get physical device surface support failed.");
                    if supports_graphic_and_surface {
                        let props = instance.get_physical_device_properties(*physical_device);
                        let name = std::str::from_utf8(
                            &*(&props.device_name
                                [..props.device_name.iter().position(|&x| x == 0).unwrap()]
                                as *const [c_char] as *const [u8]),
                        )
                        .unwrap();

                        info!("device name: {:?} info: {:?}", name, info);
                        Some(Candidate {
                            physical_device: *physical_device,
                            queue_family_index,
                            physical_device_properties: props,
                        })
                    } else {
                        None
                    }
                })
        })
        .collect::<Vec<_>>();

    let index = index.unwrap_or_else(|| {
        devices_and_queues
            .iter()
            .enumerate()
            .find(
                |(
                    _,
                    Candidate {
                        physical_device_properties,
                        ..
                    },
                )| {
                    physical_device_properties.device_type == PhysicalDeviceType::DISCRETE_GPU
                },
            )
            .map(|(i, _)| i)
            .unwrap_or(0)
    });
    let selected = &devices_and_queues[index];

    DeviceQueueFamilies {
        physical_device: selected.physical_device,
        queue_family_index: selected.queue_family_index as _,
        surface_loader,
    }
}
