/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::thread::{self, ThreadId};

use async_task::{Runnable, Task};
use futures_intrusive::timer::TimerFuture;
use futures_lite::Future;
use instant::Duration;
use parking_lot::Mutex;
use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::timer::ExecutorTimer;

use super::event::ExecutorEvent;

/// Handle task spawning and timer
#[derive(Debug)]
pub struct ExecutorHandle {
    thread_id: ThreadId,
    proxy: Mutex<EventLoopProxy<ExecutorEvent>>,

    pub(super) timer: ExecutorTimer,
}

impl ExecutorHandle {
    pub(crate) fn new(event_loop: &EventLoop<ExecutorEvent>) -> Self {
        Self {
            thread_id: thread::current().id(),
            proxy: Mutex::new(event_loop.create_proxy()),

            timer: ExecutorTimer::new(),
        }
    }

    /// Exit event loop with exit code
    pub async fn exit(&self) -> ! {
        self.proxy.lock().send_event(ExecutorEvent::Exit).unwrap();
        futures_lite::future::pending().await
    }

    /// Create Future waiting for given duration.
    pub fn wait(&self, delay: Duration) -> TimerFuture {
        let fut = self.timer.delay(delay);

        self.proxy.lock().send_event(ExecutorEvent::Wake).unwrap();

        fut
    }

    /// Create Future waiting for given timestamp
    pub fn wait_deadline(&self, timestamp: u64) -> TimerFuture {
        let fut = self.timer.deadline(timestamp);

        self.proxy.lock().send_event(ExecutorEvent::Wake).unwrap();

        fut
    }

    /// Spawn a new task, running on runtime thread
    ///
    /// Because it can be called on outside of runtime thread, the Future and its output must be [`Send`]
    pub fn spawn<Fut>(&self, fut: Fut) -> Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        // SAFETY: Future and its output is both Send and 'static
        unsafe { self.spawn_unchecked(fut) }
    }

    /// Spawn and run new task, on runtime thread.
    ///
    /// Unlike `ExecutorHandle::spawn` this method check if this method called on runtime's thread and will panic if it didn't.
    /// Therefore the Future and its output does not need to be [`Send`]
    pub fn spawn_local<Fut>(&self, fut: Fut) -> Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        if thread::current().id() != self.thread_id {
            panic!("Cannot call spawn_local outside of event loop thread");
        }

        // SAFETY: Future runs on same thread and its output is 'static
        unsafe { self.spawn_unchecked(fut) }
    }

    /// Spawn and run new task, without checking Future and its output's bound.
    ///
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

    /// # Safety
    /// See [`ExecutorHandle::spawn_unchecked`]
    pub(super) unsafe fn spawn_raw_unchecked<Fut>(&self, fut: Fut) -> (Runnable, Task<Fut::Output>)
    where
        Fut: Future,
    {
        let proxy = self.proxy.lock().clone();

        async_task::spawn_unchecked(fut, move |runnable| {
            let _ = proxy.send_event(ExecutorEvent::PollTask(runnable));
        })
    }
}
