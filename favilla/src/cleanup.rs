use crate::cleanup_queue::CleanupQueue;
use ash::vk;

pub trait Cleanup {
    fn queue(self, queue: &mut CleanupQueue);
}

impl Cleanup for vk::Buffer {
    fn queue(self, queue: &mut CleanupQueue) {
        queue.queue_buffer(self);
    }
}

impl Cleanup for vk::DeviceMemory {
    fn queue(self, queue: &mut CleanupQueue) {
        queue.queue_memory(self);
    }
}

impl<T> Cleanup for crate::buffer::VulkanBuffer<T> {
    fn queue(self, queue: &mut CleanupQueue) {
        self.buffer.queue(queue);
    }
}

impl<T: Copy> Cleanup for crate::buffer::StagingBuffer<T> {
    fn queue(self, queue: &mut CleanupQueue) {
        self.buffer.queue(queue);
    }
}

impl<T> Cleanup for crate::buffer::VulkanBufferWithDedicatedAllocation<T> {
    fn queue(self, queue: &mut CleanupQueue) {
        self.buffer.queue(queue);
        self.memory.queue(queue);
    }
}

impl<T: Copy> Cleanup for crate::buffer::StagingBufferWithDedicatedAllocation<T> {
    fn queue(self, queue: &mut CleanupQueue) {
        self.buffer.queue(queue);
        self.memory.queue(queue);
    }
}
