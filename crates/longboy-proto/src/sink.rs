pub trait Sink
{
    fn handle(&mut self, input: &[u8]);
}
