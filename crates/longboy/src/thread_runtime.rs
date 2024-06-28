use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{Builder, JoinHandle},
    time::{Duration, Instant},
};

use crate::{Runtime, RuntimeTask};

pub struct ThreadRuntime
{
    handles: Vec<JoinHandle<()>>,
    cancelled: Arc<AtomicBool>,
}

struct Task
{
    task: Box<dyn RuntimeTask>,
    cancelled: Arc<AtomicBool>,
}

impl ThreadRuntime
{
    pub fn new(cancelled: Arc<AtomicBool>) -> Self
    {
        Self {
            handles: Vec::new(),
            cancelled: cancelled,
        }
    }
}

impl Runtime for ThreadRuntime
{
    fn running(&self) -> bool
    {
        !self.cancelled.load(Ordering::Relaxed)
    }

    fn spawn(&mut self, task: Box<dyn RuntimeTask>)
    {
        let name = String::from(task.name());
        let task = Task {
            task: task,
            cancelled: self.cancelled.clone(),
        };
        self.handles
            .push(Builder::new().name(name).spawn(move || task.run()).unwrap());
    }
}

impl Drop for ThreadRuntime
{
    fn drop(&mut self)
    {
        self.cancelled.store(false, Ordering::Relaxed);
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
        while !self.cancelled.load(Ordering::Relaxed)
        {
            self.task.poll(instant.elapsed().as_millis() as u16);
            std::thread::sleep(Duration::from_millis(1));
        }

        self.cancelled.store(false, Ordering::Relaxed);
    }
}
