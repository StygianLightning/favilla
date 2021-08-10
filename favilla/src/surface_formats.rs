use ash::extensions::khr::Surface;
use ash::vk;
use ash::vk::PhysicalDevice;

pub unsafe fn find_surface_format(
    surface_loader: &Surface,
    surface: vk::SurfaceKHR,
    physical_device: PhysicalDevice,
) -> vk::SurfaceFormatKHR {
    let surface_formats = surface_loader
        .get_physical_device_surface_formats(physical_device, surface)
        .unwrap();

    println!("supported surface formats:");
    for surface_format in &surface_formats {
        println!("{:?}", surface_format);
    }

    surface_formats
        .into_iter()
        .next()
        .expect("Unable to find suitable surface format.")
}
