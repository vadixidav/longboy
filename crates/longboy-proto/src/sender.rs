use crate::Source;

pub struct Sender
{
    size: usize,
    window_size: usize,
    max_cycle: usize,

    cycle: usize,
    flags: Box<[bool]>, // window_size
    buffer: Box<[u8]>,  // cycle + (size * flags.len())

    source: Box<dyn Source>,
}

impl Sender
{
    pub fn new(size: usize, window_size: usize, source: Box<dyn Source>) -> Self
    {
        let flags_size = window_size as usize;
        let buffer_size = std::mem::size_of::<u16>() + ((size as usize) * flags_size);

        Self {
            size: size,
            window_size: window_size,
            max_cycle: ((u16::MAX as usize) / window_size) * window_size,

            cycle: 0,
            flags: (0..flags_size).map(|_| false).collect(),
            buffer: (0..buffer_size).map(|_| 0).collect(),

            source: source,
        }
    }

    pub fn cycle(&self) -> usize
    {
        self.cycle
    }

    pub fn poll_datagram(&mut self) -> Option<&[u8]>
    {
        // Record cycle.
        *self.buffer.first_chunk_mut().unwrap() = self.cycle.to_le_bytes();

        // Poll source.
        let index = self.cycle % self.window_size;
        let start = std::mem::size_of::<u16>() + (self.size * index);
        let end = start + self.size;
        self.flags[index] = self.source.poll(&mut self.buffer[start..end]);
        if !self.flags[index]
        {
            self.buffer[start..end].fill(0);
        }

        // Check for transmit and potentially advance cycle.
        match self.flags.iter().any(|flag| *flag)
        {
            true =>
            {
                self.cycle = (self.cycle + 1) % self.max_cycle;
                Some(&self.buffer)
            }
            false => None,
        }
    }
}
