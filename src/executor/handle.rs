/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use async_task::{Runnable, Task};
use futures_intrusive::timer::{Timer, TimerFuture, TimerService};
use futures_lite::Future;
use instant::Duration;
use winit::event_loop::EventLoopProxy;

use super::event::ExecutorEvent;

#[derive(Debug)]
pub struct ExecutorHandle {
    proxy: EventLoopProxy<ExecutorEvent>,

    pub(super) timer: TimerService,
}

impl ExecutorHandle {
    pub(crate) const fn new(proxy: EventLoopProxy<ExecutorEvent>, timer: TimerService) -> Self {
        Self {
            proxy,

            timer,
        }
    }

    pub async fn exit(&self, code: i32) -> ! {
        self.proxy
            .send_event(ExecutorEvent::Exit(code))
            .unwrap();
        futures_lite::future::pending().await
    }

    pub fn wait(&self, delay: Duration) -> TimerFuture {
        let fut = self.timer.delay(delay);

        self.proxy
            .send_event(ExecutorEvent::Wake)
            .unwrap();

        fut
    }

    pub fn wait_deadline(&self, timestamp: u64) -> TimerFuture {
        let fut = self.timer.deadline(timestamp);

        self.proxy
            .send_event(ExecutorEvent::Wake)
            .unwrap();

        fut
    }

    pub fn spawn<Fut>(&self, fut: Fut) -> Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        // SAFETY: Future and its output is both Send and 'static
        unsafe { self.spawn_unchecked(fut) }
    }

    /// # Safety
    /// If [`Future`] and its output is
    /// 1. not [`Send`]: Must be called on main thread.
    /// 2. non 'static: References to Future must outlive.
    pub unsafe fn spawn_unchecked<Fut>(&self, fut: Fut) -> Task<Fut::Output>
    where
        Fut: Future,
    {
        let (runnable, task) = self.spawn_raw_unchecked(fut);
        runnable.schedule();

        task
    }

    pub(super) unsafe fn spawn_raw_unchecked<Fut>(&self, fut: Fut) -> (Runnable, Task<Fut::Output>)
    where
        Fut: Future,
    {
        let proxy = self.proxy.clone();

        async_task::spawn_unchecked(fut, move |runnable| {
            let _ = proxy.send_event(ExecutorEvent::PollTask(runnable));
        })
    }
}
