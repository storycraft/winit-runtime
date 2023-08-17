/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use async_task::{Runnable, Task};
use futures_intrusive::timer::{Timer, TimerFuture, TimerService};
use futures_lite::Future;
use higher_kinded_types::ForLt;
use instant::Duration;
use parking_lot::Mutex;
use winit::{
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::EventLoopProxy,
    window::WindowId,
};

use crate::event::EventSource;

use super::event::ExecutorEvent;

#[derive(Debug)]
pub struct ExecutorHandle {
    proxy: Mutex<EventLoopProxy<ExecutorEvent>>,

    pub(super) timer: TimerService,

    pub resumed: EventSource<ForLt!(())>,
    pub suspended: EventSource<ForLt!(())>,

    pub device: EventSource<ForLt!((DeviceId, DeviceEvent))>,
    pub window: EventSource<ForLt!((WindowId, WindowEvent<'_>))>,

    pub redraw_requested: EventSource<ForLt!(WindowId)>,
}

impl ExecutorHandle {
    pub(crate) fn new(proxy: EventLoopProxy<ExecutorEvent>, timer: TimerService) -> Self {
        Self {
            proxy: Mutex::new(proxy),

            timer,

            resumed: EventSource::new(),
            suspended: EventSource::new(),

            device: EventSource::new(),
            window: EventSource::new(),

            redraw_requested: EventSource::new(),
        }
    }

    pub async fn exit(&self, code: i32) -> ! {
        self.proxy
            .lock()
            .send_event(ExecutorEvent::Exit(code))
            .unwrap();
        futures_lite::future::pending().await
    }

    pub fn wait(&self, delay: Duration) -> TimerFuture {
        let fut = self.timer.delay(delay);

        self.proxy
            .lock()
            .send_event(ExecutorEvent::TimerAdded)
            .unwrap();

        fut
    }

    pub fn wait_deadline(&self, timestamp: u64) -> TimerFuture {
        let fut = self.timer.deadline(timestamp);

        self.proxy
            .lock()
            .send_event(ExecutorEvent::TimerAdded)
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

    pub unsafe fn spawn_raw_unchecked<Fut>(&self, fut: Fut) -> (Runnable, Task<Fut::Output>)
    where
        Fut: Future,
    {
        let proxy = Mutex::new(self.proxy.lock().clone());

        async_task::spawn_unchecked(fut, move |runnable| {
            let _ = proxy.lock().send_event(ExecutorEvent::PollTask(runnable));
        })
    }
}
