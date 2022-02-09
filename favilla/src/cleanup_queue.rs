use crate::cleanup::Cleanup;
use ash::vk;

///A queue for cleaning up resources. Deletion of resources will be delayed by N
/// frames to avoid concurrency problems.
/// N should be initialized with the number of frames that can be in flight at the same time.
/// Supported resources are: ash::vk::Buffer and ash::vk::DeviceMemory.
#[derive(Debug)]
pub struct CleanupQueue {
    frame_queue: Vec<QueuedFrame>,
    current_frame_index: usize,
}

#[derive(Debug)]
struct QueuedFrame {
    buffers: Vec<vk::Buffer>,
    memory: Vec<vk::DeviceMemory>,
}

impl CleanupQueue {
    pub fn new(num_frames: usize) -> Self {
        Self {
            frame_queue: (0..num_frames).map(|_| QueuedFrame::new()).collect(),
            current_frame_index: 0,
        }
    }

    fn num_frames(&self) -> usize {
        self.frame_queue.len()
    }

    fn get_current_frame_index(&self) -> usize {
        (self.current_frame_index + self.num_frames() - 1) % self.num_frames()
    }

    pub fn queue_buffer(&mut self, buffer: vk::Buffer) {
        let current_frame_index = self.get_current_frame_index();
        self.frame_queue[current_frame_index].push_buffer(buffer)
    }

    pub fn queue_memory(&mut self, memory: vk::DeviceMemory) {
        let current_frame_index = self.get_current_frame_index();
        self.frame_queue[current_frame_index].push_memory(memory)
    }

    pub fn queue(&mut self, resource: impl Cleanup) {
        resource.queue(self);
    }

    /// Ticks the Cleanup Queue and deletes all resources that have been ticked `num_frames` times,
    /// # Safety
    /// Resources must be OK to free.
    pub unsafe fn tick(&mut self, device: &ash::Device) {
        let index = self.current_frame_index;

        self.frame_queue[index].destroy(device);

        self.current_frame_index = (self.current_frame_index + 1) % self.num_frames()
    }

    /// Cleans up all resources immediately.
    /// # Safety
    /// All resources must be OK to free.
    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        for frame in &mut self.frame_queue {
            frame.destroy(device);
        }
    }
}

impl QueuedFrame {
    fn new() -> Self {
        Self {
            buffers: Vec::new(),
            memory: Vec::new(),
        }
    }

    fn push_buffer(&mut self, buffer: vk::Buffer) {
        self.buffers.push(buffer);
    }

    fn push_memory(&mut self, memory: vk::DeviceMemory) {
        self.memory.push(memory);
    }

    unsafe fn destroy(&mut self, device: &ash::Device) {
        for buffer in &self.buffers {
            device.destroy_buffer(*buffer, None);
        }
        self.buffers.clear();

        for memory in &self.memory {
            device.free_memory(*memory, None);
        }
        self.memory.clear();
    }
}
