// API
mod constants;
use std::{marker::PhantomData, mem};

use generic_array::{ArrayLength, GenericArray};
use typenum::{Const, Unsigned};
use zerocopy::{FromBytes, IntoBytes};

pub use self::constants::*;

mod sender;
pub use self::sender::*;

mod receiver;
pub use self::receiver::*;

#[derive(FromBytes, IntoBytes)]
#[repr(packed)]
pub(crate) struct DatagramHeader
{
    cycle: u16,
    timestamp: u16,
}

#[derive(FromBytes, IntoBytes)]
#[repr(packed)]
pub(crate) struct Datagram<DatagramData: DatagramBundle>
{
    header: DatagramHeader,
    messages: GenericArray<DatagramData::Message, DatagramData::WindowSize>,
}

pub trait DatagramBundle
{
    type Message: Send + FromBytes + IntoBytes;
    type WindowSize: ArrayLength;
    /// This is the number of bytes used for the maximum number of copies of
    /// messages permitted
    type MaxCycle: ArrayLength;
}

pub struct DatagramTypeData<Message, WindowSize>
{
    _phantom: (PhantomData<Message>, PhantomData<WindowSize>),
}

pub const fn calc_max_cycle<WindowSize: ArrayLength>() -> usize
{
    u16::MAX as usize / WindowSize::USIZE * WindowSize::USIZE
}

pub const fn calc_datagram_size<DatagramData: DatagramBundle>() -> usize
{
    mem::size_of::<Datagram<DatagramData>>()
}

pub const fn calc_num_buffered<WindowSize: ArrayLength>() -> usize
{
    let double_window = 2 * WindowSize::USIZE;
    if double_window < 8
    {
        8
    }
    else
    {
        double_window
    }
}

impl<Message, WindowSize> DatagramBundle for DatagramTypeData<Message, WindowSize>
where
    Message: Send + FromBytes + IntoBytes,
    WindowSize: ArrayLength,
    Const<{ calc_max_cycle::<WindowSize>() }>: ArrayLength,
{
    type Message = Message;
    type WindowSize = WindowSize;
    type MaxCycle = Const<{ calc_max_cycle::<WindowSize>() }>;
}

// Internal
mod cipher;
pub(crate) use self::cipher::*;
