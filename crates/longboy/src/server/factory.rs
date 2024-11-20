pub trait Factory
where
    Self: 'static + Send,
{
    type Type: 'static + Send;

    fn invoke(&mut self, session_id: u64) -> Self::Type;
}

impl<Type, F> Factory for F
where
    Type: 'static + Send,
    F: 'static + FnMut() -> Type + Send,
{
    type Type = Type;
    fn invoke(&mut self, session_id: u64) -> Self::Type
    {
        (*self)()
    }
}
