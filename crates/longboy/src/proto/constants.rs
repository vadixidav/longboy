pub struct Constants<const SIZE: usize, const WINDOW_SIZE: usize>;

impl<const SIZE: usize, const WINDOW_SIZE: usize> Constants<SIZE, WINDOW_SIZE>
{
    pub const DATAGRAM_SIZE: usize = {
        let datagram_size = (std::mem::size_of::<u16>() * 2) + (SIZE * WINDOW_SIZE);
        assert!(datagram_size < 420);
        datagram_size
    };

    pub const MAX_CYCLE: usize = ((u16::MAX as usize) / WINDOW_SIZE) * WINDOW_SIZE;

    pub const MAX_BUFFERED: usize = match WINDOW_SIZE < 4
    {
        true => 8,
        false => WINDOW_SIZE * 2,
    };
}
