use crate::memory::find_memorytype_index;
use crate::vk_engine::VulkanEngine;

use ash::util::Align;
use ash::vk::{Buffer, BufferCopy, DeviceMemory};
use ash::{vk, Device};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::align_of;
use tracing::{event, Level};

pub struct VulkanBuffer<T> {
    pub buffer: Buffer,
    pub memory_flags: vk::MemoryPropertyFlags,
    pub device_size: u64,
    pub length: u64,
    phantom: PhantomData<T>,
}

impl<T> VulkanBuffer<T> {
    pub unsafe fn copy(
        &mut self,
        vk_engine: &VulkanEngine,
        command_buffer: vk::CommandBuffer,
        dst: &mut Self,
        src_offset: u64,
        dst_offset: u64,
        length: u64,
    ) -> Result<(), ()>
    where
        T: Copy,
    {
        if self.length < length || dst.length < length {
            Err(())
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
            phantom: PhantomData::default(),
        }
    }

    pub unsafe fn get_memory_requirements(&self, device: &Device) -> vk::MemoryRequirements {
        device.get_buffer_memory_requirements(self.buffer)
    }

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

    pub unsafe fn destroy(&mut self, vk_engine: &VulkanEngine) {
        vk_engine.device.destroy_buffer(self.buffer, None);
    }
}

pub struct VulkanBufferWithDedicatedAllocation<T> {
    pub buffer: VulkanBuffer<T>,
    pub memory: DeviceMemory,
}

impl<T> VulkanBufferWithDedicatedAllocation<T> {
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

        let memory_type_index = find_memorytype_index(
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
        buffer.bind_memory(&vk_engine, memory, 0);

        Self { memory, buffer }
    }

    pub unsafe fn destroy(&mut self, vk_engine: &VulkanEngine) {
        vk_engine.device.free_memory(self.memory, None);
        vk_engine.device.destroy_buffer(self.buffer.buffer, None);
    }
}

pub struct StagingBuffer<T: Copy> {
    pub buffer: VulkanBuffer<T>,
    pub buffer_ptr: *mut c_void,
}

impl<T: Copy> StagingBuffer<T> {
    pub unsafe fn write(&mut self, data: &[T]) {
        let mut slice = Align::new(
            self.buffer_ptr,
            align_of::<T>() as vk::DeviceSize,
            (data.len() * std::mem::size_of::<T>()) as _,
        );
        slice.copy_from_slice(data);
    }

    pub unsafe fn destroy(&mut self, vk_engine: &VulkanEngine) {
        self.buffer.destroy(vk_engine);
    }

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
            .unwrap();

        Self { buffer, buffer_ptr }
    }
}

pub struct StagingBufferWithDedicatedAllocation<T: Copy> {
    pub buffer: StagingBuffer<T>,
    pub memory: DeviceMemory,
}

impl<T: Copy> StagingBufferWithDedicatedAllocation<T> {
    pub unsafe fn destroy(&mut self, vk_engine: &VulkanEngine) {
        vk_engine.device.free_memory(self.memory, None);
        vk_engine
            .device
            .destroy_buffer(self.buffer.buffer.buffer, None);
    }

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
