use std::{
    thread::{Builder, JoinHandle},
    time::{Duration, Instant},
};

use tokio_util::sync::CancellationToken;

use crate::{Runtime, RuntimeTask};

pub struct ThreadRuntime
{
    handles: Vec<JoinHandle<()>>,
    cancellation_token: CancellationToken,
}

struct Task
{
    task: Box<dyn RuntimeTask>,
    cancellation_token: CancellationToken,
}

impl ThreadRuntime
{
    pub fn new(cancellation_token: CancellationToken) -> Self
    {
        Self {
            handles: Vec::new(),
            cancellation_token,
        }
    }
}

impl Runtime for ThreadRuntime
{
    fn running(&self) -> bool
    {
        !self.cancellation_token.is_cancelled()
    }

    fn spawn(&mut self, task: Box<dyn RuntimeTask>)
    {
        let name = String::from(task.name());
        let task = Task {
            task,
            cancellation_token: self.cancellation_token.clone(),
        };
        self.handles
            .push(Builder::new().name(name).spawn(move || task.run()).unwrap());
    }
}

impl Drop for ThreadRuntime
{
    fn drop(&mut self)
    {
        self.cancellation_token.cancel();
        for handle in self.handles.drain(..).rev()
        {
            handle.join().unwrap();
        }
    }
}

impl Task
{
    fn run(mut self)
    {
        let instant = Instant::now();
        while !self.cancellation_token.is_cancelled()
        {
            self.task.poll(instant.elapsed().as_millis() as u16);
            std::thread::sleep(Duration::from_millis(1));
        }

        self.cancellation_token.cancel();
    }
}
