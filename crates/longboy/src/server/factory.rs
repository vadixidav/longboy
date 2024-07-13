pub trait Factory
where
    Self: 'static + Send,
{
    type Type: 'static + Send;

    fn invoke(&mut self, session_id: u64) -> Self::Type;
}
