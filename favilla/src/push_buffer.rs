pub struct PushBuffer<T: Copy + Default> {
    pub data: Vec<T>,
    pub(crate) length: usize,
}

impl<T: Copy + Default> PushBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            data: vec![Default::default(); capacity],
            length: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn start_pass(&mut self, start_index: usize) -> Result<PushBufferPass<'_, T>, ()> {
        if start_index >= self.capacity() {
            Err(())
        } else {
            Ok(PushBufferPass::new(self, start_index))
        }
    }
}

pub struct PushBufferPass<'a, T>
where
    T: Copy + Default,
{
    push_buffer: &'a mut PushBuffer<T>,
    index: usize,
}

impl<'a, T> PushBufferPass<'a, T>
where
    T: Copy + Default,
{
    pub fn new(push_buffer: &'a mut PushBuffer<T>, start_index: usize) -> Self {
        push_buffer.length = start_index;
        Self {
            push_buffer,
            index: start_index,
        }
    }

    fn capacity(&self) -> usize {
        self.push_buffer.data.capacity()
    }

    pub fn push(&mut self, element: T) {
        if self.index == self.capacity() {
            self.push_buffer
                .data
                .resize_with(2 * self.capacity(), Default::default);
        }

        self.push_buffer.data[self.index] = element;
        self.index += 1;
    }

    pub fn finish(self) {
        self.push_buffer.length = self.index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_buffer_grows() -> Result<(), ()> {
        let mut push_buffer = PushBuffer::<u32>::new(4);
        assert_eq!(push_buffer.len(), 0);
        assert_eq!(push_buffer.capacity(), 4);

        let mut pass = push_buffer.start_pass(0)?;
        for i in 0..8 {
            pass.push(i + 1);
        }

        pass.finish();

        assert_eq!(push_buffer.len(), 8);
        assert_eq!(push_buffer.data, (1..9).collect::<Vec<_>>());

        let mut pass = push_buffer.start_pass(0)?;
        pass.push(42);
        pass.finish();

        assert_eq!(push_buffer.len(), 1);
        assert_eq!(push_buffer.data[0], 42);

        let mut pass = push_buffer.start_pass(1)?;
        pass.push(43);
        pass.finish();

        assert_eq!(push_buffer.len(), 2);
        assert_eq!(&push_buffer.data[0..push_buffer.len()], &[42, 43]);

        let err = push_buffer.start_pass(420);
        assert!(err.is_err());

        let pass = push_buffer.start_pass(0)?;
        pass.finish();

        assert_eq!(push_buffer.len(), 0);

        Ok(())
    }
}
