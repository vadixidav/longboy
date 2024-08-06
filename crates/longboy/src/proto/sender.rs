use zerocopy::{AsBytes, FromBytes, FromZeroes};

use crate::{Cipher, Constants};

#[derive(FromZeroes, FromBytes, AsBytes)]
#[repr(packed)]
struct SendDatagram<SourceType: Source, const WINDOW_SIZE: usize>
{
    cycle: u16,
    timestamp: u16,
    messages: [SourceType::Message; WINDOW_SIZE],
}

pub trait Source: Send + 'static
{
    type Message: FromBytes + AsBytes;

    fn poll(&mut self) -> Option<Self::Message>;
}

pub struct Sender<SourceType: Source, const WINDOW_SIZE: usize>
{
    source: SourceType,
    cipher: Cipher,

    cycle: usize,
    flags: [bool; WINDOW_SIZE],
    datagram: SendDatagram<SourceType, WINDOW_SIZE>,
}

impl<SourceType: Source, const WINDOW_SIZE: usize> Sender<SourceType, WINDOW_SIZE>
{
    pub fn new(cipher_key: u64, source: SourceType) -> Self
    {
        Self {
            source,
            cipher: Cipher::new(cipher_key),

            cycle: 0,
            flags: [false; WINDOW_SIZE],
            datagram: SendDatagram {
                cycle: 0,
                timestamp: 0,
                messages: [],
            },
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
