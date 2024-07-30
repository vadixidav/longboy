use crate::{Cipher, Constants};

pub struct Sender<SourceType, const SIZE: usize, const WINDOW_SIZE: usize>
where
    [(); <Constants<SIZE, WINDOW_SIZE>>::DATAGRAM_SIZE]:,
{
    source: SourceType,
    cipher: Cipher<SIZE>,

    cycle: usize,
    flags: [bool; WINDOW_SIZE],
    buffer: [u8; <Constants<SIZE, WINDOW_SIZE>>::DATAGRAM_SIZE],
}

pub trait Source<const SIZE: usize>
where
    Self: 'static + Send,
{
    fn poll(&mut self, buffer: &mut [u8; SIZE]) -> bool;
}

impl<SourceType, const SIZE: usize, const WINDOW_SIZE: usize> Sender<SourceType, SIZE, WINDOW_SIZE>
where
    SourceType: Source<SIZE>,
    [(); <Constants<SIZE, WINDOW_SIZE>>::DATAGRAM_SIZE]:,
{
    pub fn new(cipher_key: u64, source: SourceType) -> Self
    {
        Self {
            source,
            cipher: Cipher::new(cipher_key),

            cycle: 0,
            flags: [false; WINDOW_SIZE],
            buffer: [0; <Constants<SIZE, WINDOW_SIZE>>::DATAGRAM_SIZE],
        }
    }

    pub fn cycle(&self) -> usize
    {
        self.cycle
    }

    pub fn poll_datagram(&mut self, timestamp: u16) -> Option<&[u8; <Constants<SIZE, WINDOW_SIZE>>::DATAGRAM_SIZE]>
    {
        // Alias constants so they're less painful to read.
        #[allow(non_snake_case)]
        let MAX_CYCLE: usize = Constants::<SIZE, WINDOW_SIZE>::MAX_CYCLE;

        // Record cycle and timestamp.
        *<&mut [u8; 2]>::try_from(&mut self.buffer[0..2]).unwrap() = (self.cycle as u16).to_le_bytes();
        *<&mut [u8; 2]>::try_from(&mut self.buffer[2..4]).unwrap() = timestamp.to_le_bytes();
        self.cipher
            .encrypt_header(<&mut [u8; 4]>::try_from(&mut self.buffer[0..4]).unwrap());

        // Poll source.
        let index = self.cycle % WINDOW_SIZE;
        let start = (std::mem::size_of::<u16>() * 2) + (SIZE * index);
        let end = start + SIZE;
        match self
            .source
            .poll(<&mut [u8; SIZE]>::try_from(&mut self.buffer[start..end]).unwrap())
        {
            true =>
            {
                self.cipher
                    .encrypt_slot(<&mut [u8; SIZE]>::try_from(&mut self.buffer[start..end]).unwrap());
                self.flags[index] = true;
            }
            false =>
            {
                self.buffer[start..end].fill(0);
                self.flags[index] = false;
            }
        }
        if !self.flags[index]
        {
            todo!();
        }

        // Check for transmit and potentially advance cycle.
        match self.flags.iter().any(|flag| *flag)
        {
            true =>
            {
                self.cycle = (self.cycle + 1) % MAX_CYCLE;
                Some(&self.buffer)
            }
            false => None,
        }
    }
}
