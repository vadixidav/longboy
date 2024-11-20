use std::{marker::PhantomData, mem};

use generic_array::{ArrayLength, GenericArray};
use typenum::{Const, Unsigned};
use zerocopy::{FromBytes, FromZeros, IntoBytes};

use crate::Cipher;

use super::{calc_max_cycle, calc_num_buffered, Datagram, DatagramBundle, DatagramTypeData};

pub trait SourceBundle: 'static
{
    type Source: Source;
    type WindowSize: ArrayLength;
    type DatagramSize: ArrayLength;
    type DatagramData: DatagramBundle;
    type MessageSize: Send + ArrayLength;

    fn max_cycle() -> usize
    {
        <Self::DatagramData as DatagramBundle>::MaxCycle::USIZE
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

pub struct SourceTypeData<SourceType, WindowSize>
{
    _phantom: (PhantomData<SourceType>, PhantomData<WindowSize>),
}

impl<SourceType, WindowSize> SourceBundle for SourceTypeData<SourceType, WindowSize>
where
    SourceType: Source,
    Const<{ u16::MAX as usize / WindowSize::USIZE * WindowSize::USIZE }>: ArrayLength,
    WindowSize: ArrayLength,
    Const<{ calc_num_buffered::<WindowSize>() }>: ArrayLength,
    Const<{ mem::size_of::<SourceType::Message>() }>: ArrayLength,
    Const<{ calc_max_cycle::<WindowSize>() }>: ArrayLength,
{
    type Source = SourceType;
    type WindowSize = WindowSize;
    type DatagramSize = Const<{ u16::MAX as usize / WindowSize::USIZE * WindowSize::USIZE }>;
    type DatagramData = DatagramTypeData<SourceType::Message, WindowSize>;
    type MessageSize = Const<{ mem::size_of::<SourceType::Message>() }>;
}

pub trait Source: Send + 'static
{
    type Message: Send + FromBytes + IntoBytes;

    fn poll(&mut self) -> Option<Self::Message>;
}

pub struct Sender<SourceData: SourceBundle>
{
    source: SourceData::Source,
    cipher: Cipher<SourceData::MessageSize>,

    cycle: u16,
    flags: GenericArray<bool, SourceData::WindowSize>,
    datagram: Datagram<SourceData::DatagramData>,
}

impl<SourceData: SourceBundle> Sender<SourceData>
{
    pub fn new(cipher_key: u64, source: SourceData::Source) -> Self
    {
        Self {
            source,
            cipher: Cipher::new(cipher_key),

            cycle: 0,
            flags: GenericArray::generate(|_| false),
            datagram: SourceData::DatagramData::new_zeroed(),
        }
    }

    pub fn cycle(&self) -> usize
    {
        self.cycle
    }

    pub fn poll_datagram(&mut self, timestamp: u16) -> Option<&Datagram<SourceData::DatagramData>>
    {
        // Record cycle and timestamp.
        self.datagram.cycle = self.cycle;
        self.datagram.timestamp = timestamp;
        self.cipher
            .encrypt_header(<&mut [u8; 4]>::try_from(&mut self.buffer[0..4]).unwrap());

        // Poll source.
        let index = self.cycle % SourceData::window_size();
        let start = (std::mem::size_of::<u16>() * 2) + (SourceData::message_size() * index);
        let end = start + SourceData::message_size();
        if let Some(message) = self.source.poll()
        {
            self.datagram.messages[index] = message;
            self.cipher.encrypt_slot(&mut self.datagram.messages[index]);
            self.flags[index] = true;
        }
        else
        {
            self.buffer[start..end].fill(0);
            self.datagram.messages[index].zero();
            self.flags[index] = false;
        }

        // Check for transmit and potentially advance cycle.
        match self.flags.iter().any(|flag| *flag)
        {
            true =>
            {
                self.cycle = self.cycle.wrapping_add(1) % SourceData::max_cycle();
                Some(&self.buffer)
            }
            false => None,
        }
    }
}
