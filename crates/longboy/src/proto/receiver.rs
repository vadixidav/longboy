use std::{marker::PhantomData, mem};

use generic_array::{sequence::GenericSequence, ArrayLength, GenericArray};
use typenum::{Const, Unsigned};
use zerocopy::{FromBytes, IntoBytes};

use crate::Cipher;

use super::{calc_datagram_size, calc_max_cycle, calc_num_buffered, Datagram, DatagramBundle, DatagramTypeData};

pub trait SinkBundle
{
    type Sink: Sink;
    type WindowSize: ArrayLength;
    type NumBuffered: ArrayLength;
    type DatagramSize: ArrayLength;
    type DatagramData: DatagramBundle;
    type MessageSize: ArrayLength;

    fn max_cycle() -> usize
    {
        <Self::DatagramData as DatagramBundle>::MaxCycle::USIZE
    }

    fn num_buffered() -> usize
    {
        Self::NumBuffered::USIZE
    }

    fn window_size() -> usize
    {
        Self::WindowSize::USIZE
    }

    fn message_size() -> usize
    {
        Self::MessageSize::USIZE
    }
}

pub struct SinkTypeData<SinkType, DatagramData>
{
    _phantom: (PhantomData<SinkType>, PhantomData<DatagramData>),
}

impl<SinkType: Sink, DatagramData: DatagramBundle> SinkBundle for SinkTypeData<SinkType, DatagramData>
where
    Const<{ calc_datagram_size::<DatagramData>() }>: ArrayLength,
    Const<{ calc_max_cycle::<DatagramData::WindowSize>() }>: ArrayLength,
    Const<{ calc_num_buffered::<DatagramData::WindowSize>() }>: ArrayLength,
    Const<{ mem::size_of::<SinkType::Message>() }>: ArrayLength,
{
    type Sink = SinkType;
    type WindowSize = DatagramData::WindowSize;
    type NumBuffered = Const<{ calc_num_buffered::<DatagramData::WindowSize>() }>;
    type DatagramSize = Const<{ calc_datagram_size::<DatagramData>() }>;
    type DatagramData = DatagramTypeData<SinkType::Message, DatagramData::WindowSize>;
    type MessageSize = Const<{ mem::size_of::<SinkType::Message>() }>;
}

pub struct Receiver<SinkData: SinkBundle>
{
    sink: SinkData::Sink,
    cipher: Cipher<SinkData::MessageSize>,

    cycle: usize,
    flags: GenericArray<bool, SinkData::NumBuffered>,
}

pub trait Sink: Send + 'static
{
    type Message: Send + FromBytes + IntoBytes;
    fn handle(&mut self, message: Self::Message);
}

impl<SinkData: SinkBundle> Receiver<SinkData>
{
    pub fn new(cipher_key: u64, sink: SinkData::Sink) -> Self
    {
        Self {
            sink,
            cipher: Cipher::new(cipher_key),

            cycle: 0,
            flags: GenericArray::generate(|_| false),
        }
    }

    pub fn cycle(&self) -> usize
    {
        self.cycle
    }

    pub fn handle_datagram(&mut self, timestamp: u16, datagram: Datagram<SinkData::DatagramData>)
    {
        // Grab cycle and timestamp.
        self.cipher.decrypt_header(self.datagram.header.as_bytes_mut());

        // Calculate diff for cycle and timestamp.
        let cycle_diff = ((datagram.cycle as usize + SinkData::max_cycle()) - self.cycle) % SinkData::max_cycle();
        let timestamp_diff = ((datagram.timestamp + u16::MAX) - timestamp) % u16::MAX;

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
        if cycle_diff > std::cmp::min(8, SinkData::window_size() + 1)
        {
            // soft warning
        }
        if cycle_diff > SinkData::num_buffered()
        {
            // hard warning
            for _ in 0..(cycle_diff - SinkData::num_buffered())
            {
                let index = self.cycle % SinkData::num_buffered();
                self.flags[index] = false;
                self.cycle = (self.cycle + 1) % SinkData::max_cycle();
            }
        }

        // Sink input.
        for i in 0..SinkData::window_size()
        {
            let cycle_i = ((datagram.cycle + SinkData::max_cycle()) - i) % SinkData::max_cycle();

            // If we're before local cycle, early out.  This is effectively checking for distance
            // being out of the buffer's size, which is only possible if before because we've
            // already adanced the local cycle to catch up, if applicable.
            if ((cycle_i + SinkData::max_cycle()) - self.cycle) % SinkData::max_cycle() > SinkData::num_buffered()
            {
                break;
            }

            let source_index = cycle_i % SinkData::window_size();
            let destination_index = cycle_i % SinkData::num_buffered();

            if !self.flags[destination_index]
            {
                let start = (std::mem::size_of::<u16>() * 2) + (SinkData::message_size() * source_index);
                let end = start + SinkData::message_size();
                if (&datagram[start..end]).try_into().unwrap() != [0; SinkData::message_size()]
                {
                    self.cipher.decrypt_slot(
                        <&mut [u8; SinkData::message_size()]>::try_from(&mut datagram[start..end]).unwrap(),
                    );
                    self.sink.handle((&datagram[start..end]).try_into().unwrap());
                }
                self.flags[destination_index] = true;
            }
        }

        // Advance cycles.
        loop
        {
            let index = self.cycle % SinkData::num_buffered();
            if !self.flags[index]
            {
                break;
            }
            self.flags[index] = false;
            self.cycle = (self.cycle + 1) % SinkData::max_cycle();
        }
    }
}
