use ash::vk::{DeviceMemory, DeviceSize, MemoryRequirements};
use ash::{vk, Device};
use thiserror::Error;

/// A simple linear allocator.
#[derive(Debug)]
pub struct LinearAllocator {
    memory: DeviceMemory,
    size: u64,
    free_section_start: u64,
}

/// An allocation within a larger chunk of allocated memory.
#[derive(Debug)]
pub struct SubAllocation {
    pub memory: DeviceMemory,
    pub offset: u64,
    pub size: u64,
}

#[derive(Error, Debug)]
pub enum SubAllocationError {
    #[error("Out of memory")]
    OutOfMemory,
}

pub fn get_aligned_offset(offset: u64, alignment: u64) -> u64 {
    let misalignment = offset % alignment;
    let padding = if misalignment == 0 {
        0
    } else {
        alignment - misalignment
    };
    offset + padding
}

impl LinearAllocator {
    /// Create a new linear allocator with the given memory size and memory type index.
    pub unsafe fn new(
        device: &Device,
        size: DeviceSize,
        memory_type_index: u32,
    ) -> Result<Self, vk::Result> {
        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: size,
            memory_type_index,
            ..Default::default()
        };

        let memory = device.allocate_memory(&allocate_info, None)?;

        Ok(Self {
            memory,
            size,
            free_section_start: 0,
        })
    }

    /// Try to get a free chunk of memory from the allocator.
    pub unsafe fn allocate(
        &mut self,
        memory_req: MemoryRequirements,
    ) -> Result<SubAllocation, SubAllocationError> {
        let offset = get_aligned_offset(self.free_section_start, memory_req.alignment);
        let new_start = offset + memory_req.size;

        if new_start <= self.size {
            self.free_section_start = new_start;
            Ok(SubAllocation {
                memory: self.memory,
                offset,
                size: memory_req.size,
            })
        } else {
            Err(SubAllocationError::OutOfMemory)
        }
    }

    /// Frees the allocator's memory.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.free_memory(self.memory, None);
    }
}
