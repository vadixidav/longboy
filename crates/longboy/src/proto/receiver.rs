use zerocopy::{AsBytes, FromBytes};

use crate::{Cipher, Constants};

#[derive(FromZeroes, FromBytes, AsBytes)]
#[repr(packed)]
struct Datagram<SinkType: Sink, const WINDOW_SIZE: usize>
{
    cycle: u16,
    timestamp: u16,
    messages: [SinkType::Message; WINDOW_SIZE],
}

pub struct Receiver<SinkType: Sink, const WINDOW_SIZE: usize>
{
    sink: SinkType,
    cipher: Cipher,

    cycle: usize,
    flags: [bool; <Constants<SIZE, WINDOW_SIZE>>::MAX_BUFFERED],
}

pub trait Sink: Send + 'static
{
    type Message: FromBytes + AsBytes;
    fn handle(&mut self, message: Self::Message);
}

impl<SinkType: Sink, const WINDOW_SIZE: usize> Receiver<SinkType, WINDOW_SIZE>
{
    pub fn new(cipher_key: u64, sink: SinkType) -> Self
    {
        Self {
            sink,
            cipher: Cipher::new(cipher_key),

            cycle: 0,
            flags: [false; <Constants<SIZE, WINDOW_SIZE>>::MAX_BUFFERED],
        }
    }

    pub fn cycle(&self) -> usize
    {
        self.cycle
    }

    pub fn handle_datagram(
        &mut self,
        timestamp: u16,
        datagram: &mut [u8; <Constants<SIZE, WINDOW_SIZE>>::DATAGRAM_SIZE],
    )
    {
        // Alias constants so they're less painful to read.
        #[allow(non_snake_case)]
        let MAX_CYCLE: usize = Constants::<SIZE, WINDOW_SIZE>::MAX_CYCLE;
        #[allow(non_snake_case)]
        let MAX_BUFFERED: usize = Constants::<SIZE, WINDOW_SIZE>::MAX_BUFFERED;

        // Grab cycle and timestamp.
        self.cipher.decrypt_header((&mut datagram[0..4]).try_into().unwrap());
        let datagram_cycle = u16::from_le_bytes((&datagram[0..2]).try_into().unwrap()) as usize;
        let datagram_timestamp = u16::from_le_bytes((&datagram[2..4]).try_into().unwrap());

        // Calculate diff for cycle and timestamp.
        let cycle_diff = ((datagram_cycle + MAX_CYCLE) - self.cycle) % MAX_CYCLE;
        let timestamp_diff = ((datagram_timestamp + u16::MAX) - timestamp) % u16::MAX;

        // Check for bad datagrams or late datagrams that are already processed.  Because
        // we ensure only a positive diff, this is done by checking for any values greater
        // that a certain threshold.
        if cycle_diff > 256 || timestamp_diff > 2048
        {
            // Bad datagram or already received.
            return;
        }

        // Check for late or missing packets from between local cycle and the datagram
        // cycle just received.
        if cycle_diff > std::cmp::min(8, WINDOW_SIZE + 1)
        {
            // soft warning
        }
        if cycle_diff > MAX_BUFFERED
        {
            // hard warning
            for _ in 0..(cycle_diff - MAX_BUFFERED)
            {
                let index = self.cycle % MAX_BUFFERED;
                self.flags[index] = false;
                self.cycle = (self.cycle + 1) % MAX_CYCLE;
            }
        }

        // Sink input.
        for i in 0..WINDOW_SIZE
        {
            let cycle_i = ((datagram_cycle + MAX_CYCLE) - i) % MAX_CYCLE;

            // If we're before local cycle, early out.  This is effectively checking for distance
            // being out of the buffer's size, which is only possible if before because we've
            // already adanced the local cycle to catch up, if applicable.
            if ((cycle_i + MAX_CYCLE) - self.cycle) % MAX_CYCLE > MAX_BUFFERED
            {
                break;
            }

            let source_index = cycle_i % WINDOW_SIZE;
            let destination_index = cycle_i % MAX_BUFFERED;

            if !self.flags[destination_index]
            {
                let start = (std::mem::size_of::<u16>() * 2) + (SIZE * source_index);
                let end = start + SIZE;
                if (&datagram[start..end]).try_into().unwrap() != [0; SIZE]
                {
                    self.cipher
                        .decrypt_slot(<&mut [u8; SIZE]>::try_from(&mut datagram[start..end]).unwrap());
                    self.sink.handle((&datagram[start..end]).try_into().unwrap());
                }
                self.flags[destination_index] = true;
            }
        }

        // Advance cycles.
        loop
        {
            let index = self.cycle % MAX_BUFFERED;
            if !self.flags[index]
            {
                break;
            }
            self.flags[index] = false;
            self.cycle = (self.cycle + 1) % MAX_CYCLE;
        }
    }
}
