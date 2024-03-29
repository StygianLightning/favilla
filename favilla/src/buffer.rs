use crate::memory::find_memory_type_index;
use crate::vk_engine::VulkanEngine;

use ash::util::Align;
use ash::vk::{Buffer, BufferCopy, DeviceMemory};
use ash::{vk, Device};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::align_of;
use thiserror::Error;
use tracing::{event, Level};

/// Typed Vulkan buffer.
pub struct VulkanBuffer<T> {
    pub buffer: Buffer,
    pub memory_flags: vk::MemoryPropertyFlags,
    pub device_size: u64,
    pub length: u64,
    phantom: PhantomData<T>,
}

#[derive(Copy, Clone, Error, Debug)]
pub enum BufferCopyError {
    #[error("")]
    SpecifiedLengthExceedsBounds,
}

impl<T> VulkanBuffer<T> {
    /// Copy data from one Vulkan buffer to another.
    /// # Safety
    /// Requires correct usage flags.
    pub unsafe fn copy(
        &mut self,
        vk_engine: &VulkanEngine,
        command_buffer: vk::CommandBuffer,
        dst: &mut Self,
        src_offset: u64,
        dst_offset: u64,
        length: u64,
    ) -> Result<(), BufferCopyError>
    where
        T: Copy,
    {
        if self.length < length || dst.length < length {
            Err(BufferCopyError::SpecifiedLengthExceedsBounds)
        } else {
            vk_engine.device.cmd_copy_buffer(
                command_buffer,
                self.buffer,
                dst.buffer,
                &[BufferCopy::builder()
                    .src_offset(src_offset)
                    .dst_offset(dst_offset)
                    .size(length * std::mem::size_of::<T>() as u64)
                    .build()],
            );
            Ok(())
        }
    }

    /// Creates a new `VulkanBuffer`. Panics if creation fails.
    /// # Safety
    /// Requires a valid device and sufficient device memory.
    pub unsafe fn new(
        vk_engine: &VulkanEngine,
        length: u64,
        usage: vk::BufferUsageFlags,
        sharing_mode: vk::SharingMode,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Self {
        let size = length * std::mem::size_of::<T>() as u64;
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode,
            ..Default::default()
        };

        let buffer = vk_engine.device.create_buffer(&buffer_info, None).unwrap();

        Self {
            buffer,
            device_size: size,
            length,
            memory_flags: memory_property_flags,
            phantom: PhantomData {},
        }
    }

    /// Get the memory requirements for `self`.
    /// # Safety
    /// Buffer and device have to be valid.
    pub unsafe fn get_memory_requirements(&self, device: &Device) -> vk::MemoryRequirements {
        device.get_buffer_memory_requirements(self.buffer)
    }

    /// Bind memory to the Vulkan buffer held by `self`.
    /// # Safety
    /// Requires a valid device and memory region.
    pub unsafe fn bind_memory(
        &mut self,
        engine: &VulkanEngine,
        buffer_memory: DeviceMemory,
        offset: vk::DeviceSize,
    ) {
        engine
            .device
            .bind_buffer_memory(self.buffer, buffer_memory, offset)
            .expect("Binding memory buffer failed");
    }

    /// Frees the buffer resource held by `self`.
    /// # Safety
    /// Buffer must not be in use anymore.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_buffer(self.buffer, None);
    }
}

/// A Vulkan buffer with a dedicated memory allocation.
pub struct VulkanBufferWithDedicatedAllocation<T> {
    pub buffer: VulkanBuffer<T>,
    pub memory: DeviceMemory,
}

impl<T> VulkanBufferWithDedicatedAllocation<T> {
    /// Allocates a new buffer.
    /// # Safety
    /// Requires a valid device with sufficient memory.
    pub unsafe fn allocate(
        vk_engine: &VulkanEngine,
        length: u64,
        usage: vk::BufferUsageFlags,
        sharing_mode: vk::SharingMode,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Self {
        let mut buffer = VulkanBuffer::new(
            vk_engine,
            length,
            usage,
            sharing_mode,
            memory_property_flags,
        );
        let mem_req = buffer.get_memory_requirements(&vk_engine.device);

        let memory_type_index = find_memory_type_index(
            &mem_req,
            &vk_engine.device_memory_properties,
            memory_property_flags,
        );
        event!(
            Level::DEBUG,
            "Selected memory type index for buffer: {}",
            memory_type_index
        );

        let memory = vk_engine.allocate_memory(mem_req, memory_type_index);
        buffer.bind_memory(vk_engine, memory, 0);

        Self { memory, buffer }
    }

    /// Frees the buffer and memory resources held by `self`.
    /// # Safety
    /// The memory and buffer must not be used anymore.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.free_memory(self.memory, None);
        device.destroy_buffer(self.buffer.buffer, None);
    }
}

/// A wrapper for a staging buffer. Holds a `VulkanBuffer<T>` and a pointer to the mapped memory.
pub struct StagingBuffer<T: Copy> {
    pub buffer: VulkanBuffer<T>,
    pub buffer_ptr: *mut T,
}

impl<T: Copy> StagingBuffer<T> {
    /// Write the data to the staging buffer.
    /// # Safety
    /// The buffer must not be accessed without proper synchronisation.
    pub unsafe fn write(&mut self, data: &[T], offset_in_element_count: usize) {
        let ptr = self.buffer_ptr.add(offset_in_element_count);
        let len = data.len();
        ptr.copy_from_nonoverlapping(data.as_ptr(), len);
    }

    /// Frees the buffer resource held by `self`.
    /// # Safety
    /// The buffer must not be used anymore.
    pub unsafe fn destroy(&mut self, device: &Device) {
        self.buffer.destroy(device);
    }

    /// Creates a new `StagingBuffer<T>`. Maps the buffer memory for writing; it is never unmapped.
    /// # Safety
    /// Requires sufficient memory
    pub unsafe fn new(
        vk_engine: &VulkanEngine,
        buffer: VulkanBuffer<T>,
        memory: DeviceMemory,
        offset: vk::DeviceSize,
    ) -> Self {
        let buffer_ptr = vk_engine
            .device
            .map_memory(
                memory,
                offset,
                buffer.device_size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap() as *mut T;

        Self { buffer, buffer_ptr }
    }
}

/// A staging buffer with a dedicated allocation.
pub struct StagingBufferWithDedicatedAllocation<T: Copy> {
    pub buffer: StagingBuffer<T>,
    pub memory: DeviceMemory,
}

impl<T: Copy> StagingBufferWithDedicatedAllocation<T> {
    /// Frees the buffer and memory resources held by `self`.
    /// # Safety
    /// The buffer and memory must not be used anymore.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.free_memory(self.memory, None);
        device.destroy_buffer(self.buffer.buffer.buffer, None);
    }

    /// Allocates a new staging buffer.
    /// # Safety
    /// Requires a valid device with sufficient memory.
    pub unsafe fn allocate(
        vk_engine: &VulkanEngine,
        length: u64,
        usage: vk::BufferUsageFlags,
        sharing_mode: vk::SharingMode,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Self {
        let dedicated_allocated_buffer = VulkanBufferWithDedicatedAllocation::allocate(
            vk_engine,
            length,
            usage,
            sharing_mode,
            memory_property_flags,
        );

        Self {
            memory: dedicated_allocated_buffer.memory,
            buffer: StagingBuffer::new(
                vk_engine,
                dedicated_allocated_buffer.buffer,
                dedicated_allocated_buffer.memory,
                0,
            ),
        }
    }
}
