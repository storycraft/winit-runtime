/*
 * Created on Mon Aug 07 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::sync::OnceLock;

use async_task::{Runnable, Task};
use futures_intrusive::timer::{Timer, TimerFuture, TimerService};
use futures_lite::Future;
use instant::Duration;
use parking_lot::Mutex;
use scoped_tls_hkt::scoped_thread_local;
use winit::{
    event::{Event, DeviceId, DeviceEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
};

use crate::{event::EventSource, timer};

pub type EventLoopTarget = EventLoopWindowTarget<ExecutorEvent>;

static HANDLE: OnceLock<ExecutorHandle> = OnceLock::new();

pub fn executor_handle() -> &'static ExecutorHandle {
    HANDLE.get().expect("Executor is not started")
}

scoped_thread_local!(static EL_TARGET: EventLoopTarget);

pub fn with_eventloop_target<R>(func: impl FnOnce(&EventLoopTarget) -> R) -> R {
    EL_TARGET.with(func)
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ExecutorEvent {
    PollTask(Runnable),
    TimerAdded,
    Exit(i32),
}

struct Executor {
    handle: &'static ExecutorHandle,
}

impl Executor {
    fn on_event(
        &mut self,
        event: Event<ExecutorEvent>,
        target: &EventLoopTarget,
        control_flow: &mut ControlFlow,
    ) {
        match event {
            Event::UserEvent(ExecutorEvent::TimerAdded) => {}

            Event::UserEvent(ExecutorEvent::PollTask(runnable)) => {
                EL_TARGET.set(&target, move || runnable.run());
            }

            Event::NewEvents(_) => {
                self.handle.timer.check_expirations();
            }

            Event::UserEvent(ExecutorEvent::Exit(code)) => {
                *control_flow = ControlFlow::ExitWithCode(code);
            }

            Event::MainEventsCleared => {}

            Event::RedrawRequested(id) => {}

            Event::RedrawEventsCleared => {
                if let Some(time) = self.handle.timer.next_expiration() {
                    let now = instant::now() as u64;
                    if time > now {
                        control_flow.set_wait_timeout(Duration::from_millis(time - now));
                    } else {
                        *control_flow = ControlFlow::Poll;
                    }
                } else if *control_flow != ControlFlow::Wait {
                    *control_flow = ControlFlow::Wait;
                }
            }

            Event::DeviceEvent { device_id, event } => {
                self.handle.device.emit(&mut (device_id, event));
            }

            Event::Resumed => {
                self.handle.resumed.emit(&mut ());
            }

            Event::Suspended => {
                self.handle.suspended.emit(&mut ());
            }

            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct ExecutorHandle {
    proxy: Mutex<EventLoopProxy<ExecutorEvent>>,

    timer: TimerService,

    pub resumed: EventSource<()>,
    pub suspended: EventSource<()>,

    pub device: EventSource<(DeviceId, DeviceEvent)>,
}

impl ExecutorHandle {
    fn new(proxy: EventLoopProxy<ExecutorEvent>, timer: TimerService) -> Self {
        Self {
            proxy: Mutex::new(proxy),

            timer,

            resumed: EventSource::new(),
            suspended: EventSource::new(),

            device: EventSource::new(),
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

    unsafe fn spawn_raw_unchecked<Fut>(&self, fut: Fut) -> (Runnable, Task<Fut::Output>)
    where
        Fut: Future,
    {
        let proxy = Mutex::new(self.proxy.lock().clone());

        async_task::spawn_unchecked(fut, move |runnable| {
            let _ = proxy.lock().send_event(ExecutorEvent::PollTask(runnable));
        })
    }
}

pub fn run(main: impl Future<Output = ()> + 'static) -> ! {
    let event_loop = EventLoopBuilder::with_user_event().build();

    let proxy = event_loop.create_proxy();

    HANDLE
        .set(ExecutorHandle::new(proxy.clone(), timer::create_service()))
        .expect("This cannot be happen");

    let handle = HANDLE.get().unwrap();

    let mut executor = Executor { handle };

    let (runnable, task) = unsafe {
        handle.spawn_raw_unchecked(async move {
            main.await;
            proxy.send_event(ExecutorEvent::Exit(0)).unwrap();
        })
    };
    task.detach();

    EL_TARGET.set(&event_loop, move || runnable.run());

    event_loop
        .run(move |event, target, control_flow| executor.on_event(event, target, control_flow));
}
