pub trait Runtime
{
    fn running(&self) -> bool;

    fn spawn(&mut self, task: Box<dyn RuntimeTask>);
}

pub trait RuntimeTask
where
    Self: 'static + Send,
{
    fn name(&self) -> &str;

    fn poll(&mut self, timestamp: u16);
}
