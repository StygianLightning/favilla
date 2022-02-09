/// A growable CPU-side buffer.
/// Can be useful for vertex buffers that are updated often.
pub struct PushBuffer<T> {
    data: Vec<T>,
}

impl<T> PushBuffer<T> {
    /// Create a new buffer with the given capacity. The capacity has to be > 0.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Get the currently used length of the buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// True iff the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    /// Get the total allocated capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Start a new pass.
    pub fn start_pass(&mut self) -> PushBufferPass<'_, T> {
        self.data.clear();
        PushBufferPass::new(self)
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }
}

/// Supports writing to the underlying `PushBuffer` from a given start index.
pub struct PushBufferPass<'a, T> {
    push_buffer: &'a mut PushBuffer<T>,
}

impl<'a, T> PushBufferPass<'a, T> {
    /// Create a new pass for the given buffer from the given start index.
    pub fn new(push_buffer: &'a mut PushBuffer<T>) -> Self {
        Self { push_buffer }
    }

    /// Push a new element onto the buffer. This can override existing data or grow the buffer.
    pub fn push(&mut self, element: T) {
        self.push_buffer.data.push(element);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_buffer_grows() {
        let mut push_buffer = PushBuffer::<u32>::new(4);
        assert_eq!(push_buffer.len(), 0);
        assert_eq!(push_buffer.capacity(), 4);

        {
            let mut pass = push_buffer.start_pass();
            for i in 0..8 {
                pass.push(i + 1);
            }
        }

        assert_eq!(push_buffer.len(), 8);
        assert_eq!(push_buffer.data(), &(1..9).collect::<Vec<_>>());
        assert!(push_buffer.capacity() >= 8);

        let pass = push_buffer.start_pass();
        drop(pass);

        assert_eq!(push_buffer.len(), 0);
    }
}
