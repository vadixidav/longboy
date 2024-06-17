use crate::Sink;

pub struct Receiver
{
    size: usize,
    window_size: usize,
    max_cycle: usize,

    cycle: usize,
    flags: Box<[bool]>, // max(8, window_size * 2)

    sink: Box<dyn Sink>,
}

impl Receiver
{
    pub fn new(size: usize, window_size: usize, sink: Box<dyn Sink>) -> Self
    {
        let flags_size = std::cmp::max(8, 2 * window_size);

        Self {
            size: size,
            window_size: window_size,
            max_cycle: ((u16::MAX as usize) / window_size) * window_size,

            cycle: 0,
            flags: (0..flags_size).map(|_| false).collect(),

            sink: sink,
        }
    }

    pub fn cycle(&self) -> usize
    {
        self.cycle
    }

    pub fn handle_datagram(&mut self, datagram: &[u8])
    {
        // Grab cycle.
        let cycle = u16::from_le_bytes(*datagram.first_chunk().unwrap()) as usize;

        // Check for lost cycles.
        let mut cycle_offset = ((cycle + self.max_cycle) - self.cycle) % self.max_cycle;
        if cycle_offset > 1024
        {
            // Already received.
            return;
        }
        if cycle_offset > std::cmp::min(8, self.window_size + 1)
        {
            // soft warning
        }
        if cycle_offset > self.flags.len()
        {
            // hard warning

            while cycle_offset > self.flags.len()
            {
                let index = self.cycle % self.flags.len();
                self.flags[index] = false;
                self.cycle = (self.cycle + 1) % self.max_cycle;
                cycle_offset -= 1;
            }
        }

        // Sink input.
        for i in 0..self.window_size
        {
            let cycle_i = ((cycle + self.max_cycle) - i) % self.max_cycle;

            let source_index = cycle_i % self.window_size;
            let destination_index = cycle_i % self.flags.len();

            if !self.flags[destination_index]
            {
                let start = std::mem::size_of::<u16>() + (self.size * source_index);
                let end = start + self.size;
                self.sink.handle(&datagram[start..end]);
                self.flags[destination_index] = true;
            }
        }

        // Advance cycles.
        loop
        {
            let index = self.cycle % self.flags.len();
            if !self.flags[index]
            {
                break;
            }
            self.flags[index] = false;
            self.cycle = (self.cycle + 1) % self.max_cycle;
        }
    }
}
