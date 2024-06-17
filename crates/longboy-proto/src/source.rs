pub trait Source
{
    fn poll(&mut self, buffer: &mut [u8]) -> bool;
}
