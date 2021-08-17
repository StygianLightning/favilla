use ash::vk;

/// Find a memory type index suitable for the given requirements and flags.
/// Panics if no suitable memory type can be found.
pub fn find_memory_type_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> u32 {
    try_find_memory_type_index(memory_req, memory_prop, flags)
        .expect("Failed to find suitable memory index type")
}

/// Find a memory type index suitable for the given requirements and flags.
pub fn try_find_memory_type_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags.contains(flags)
        })
        .map(|(index, _memory_type)| index as _)
}
