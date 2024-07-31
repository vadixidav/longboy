use std::time::Duration;

use tokio::{
    select,
    task::JoinHandle,
    time::{Instant, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

use crate::{Runtime, RuntimeTask};

pub struct TokioRuntime
{
    handles: Vec<JoinHandle<()>>,
    cancellation_token: CancellationToken,
}

struct Task
{
    task: Box<dyn RuntimeTask>,
    cancellation_token: CancellationToken,
}

impl TokioRuntime
{
    pub fn new(cancellation_token: CancellationToken) -> Self
    {
        Self {
            handles: Vec::new(),
            cancellation_token,
        }
    }
}

impl Runtime for TokioRuntime
{
    fn running(&self) -> bool
    {
        !self.cancellation_token.is_cancelled()
    }

    fn spawn(&mut self, task: Box<dyn RuntimeTask>)
    {
        let task = Task {
            task,
            cancellation_token: self.cancellation_token.clone(),
        };
        self.handles.push(tokio::spawn(task.run()));

        // `Builder` API blocked on `tokio` unstable features.
        // let name = String::from(task.name());
        // self.handles
        //     .push(Builder::new().name(name).spawn(move || task.run()).unwrap());
    }
}

impl Drop for TokioRuntime
{
    fn drop(&mut self)
    {
        self.cancellation_token.cancel();
        for handle in self.handles.drain(..).rev()
        {
            handle.abort();
        }
    }
}

impl Task
{
    async fn run(mut self)
    {
        let mut interval = tokio::time::interval(Duration::from_millis(1));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut instant = Instant::now();

        loop
        {
            let tick = select! {
                tick = interval.tick() => tick,
                _ = self.cancellation_token.cancelled() => break,
            };
            let elapsed = tick.duration_since(instant);
            instant = tick;

            self.task.poll(elapsed.as_millis() as u16);
        }

        self.cancellation_token.cancel();
    }
}
