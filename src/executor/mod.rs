/*
 * Created on Mon Aug 07 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::sync::OnceLock;

use async_task::{Runnable, Task};
use flume::{Receiver, Sender};
use futures_intrusive::timer::{Timer, TimerFuture, TimerService};
use futures_lite::Future;
use instant::Duration;
use parking_lot::Mutex;
use scoped_tls_hkt::scoped_thread_local;
use winit::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
};

use crate::{
    event::{AsyncEventTarget, Subscription},
    timer,
};

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
    Wake,
    Exit(i32),
}

struct Executor {
    handle: &'static ExecutorHandle,
    task_recv: Receiver<Runnable>,
}

impl Executor {
    fn poll_tasks(&self, target: &EventLoopTarget) {
        let drain = self.task_recv.drain();
        if drain.len() == 0 {
            return;
        }

        EL_TARGET.set(target, move || {
            for runnable in drain {
                runnable.run();
            }
        });
    }

    fn on_event(
        &mut self,
        event: Event<ExecutorEvent>,
        target: &EventLoopTarget,
        control_flow: &mut ControlFlow,
    ) {
        match event {
            Event::UserEvent(ExecutorEvent::Wake) => {}

            Event::NewEvents(cause) => {
                if let StartCause::Init = cause {
                    self.poll_tasks(target);
                }

                self.handle.timer.check_expirations();
            }

            Event::UserEvent(ExecutorEvent::Exit(code)) => {
                *control_flow = ControlFlow::ExitWithCode(code);
            }

            Event::MainEventsCleared => {
                self.poll_tasks(target);
            }

            Event::RedrawRequested(id) => {

            }

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

            Event::Resumed => {
                self.handle.resumed.dispatch();
                self.poll_tasks(target);
            }

            Event::Suspended => {
                self.handle.suspended.dispatch();
                self.poll_tasks(target);
            }

            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct ExecutorHandle {
    proxy: Mutex<EventLoopProxy<ExecutorEvent>>,
    task_sender: Sender<Runnable>,

    timer: TimerService,

    resumed: AsyncEventTarget,
    suspended: AsyncEventTarget,
}

impl ExecutorHandle {
    fn new(proxy: EventLoopProxy<ExecutorEvent>, task_sender: Sender<Runnable>) -> Self {
        Self {
            proxy: Mutex::new(proxy),
            task_sender,

            timer: timer::create_service(),

            resumed: AsyncEventTarget::new(),
            suspended: AsyncEventTarget::new(),
        }
    }

    pub async fn exit(&self, code: i32) -> ! {
        self.proxy
            .lock()
            .send_event(ExecutorEvent::Exit(code))
            .unwrap();
        futures_lite::future::pending().await
    }

    pub fn resumed(&self) -> Subscription {
        self.resumed.subscribe()
    }

    pub fn suspended(&self) -> Subscription {
        self.suspended.subscribe()
    }

    pub fn wait(&self, delay: Duration) -> TimerFuture {
        let fut = self.timer.delay(delay);

        self.proxy
            .lock()
            .send_event(ExecutorEvent::Wake)
            .unwrap();

        fut
    }

    pub fn deadline(&self, timestamp: u64) -> TimerFuture {
        let fut = self.timer.deadline(timestamp);

        self.proxy
            .lock()
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
        let (runnable, task) = {
            let proxy = Mutex::new(self.proxy.lock().clone());
            let task_sender = self.task_sender.clone();

            async_task::spawn_unchecked(fut, move |runnable| {
                let _ = task_sender.send(runnable);
                let _ = proxy.lock().send_event(ExecutorEvent::Wake);
            })
        };
        runnable.schedule();

        task
    }
}

pub fn run(main: impl Future<Output = ()> + 'static) -> ! {
    let event_loop = EventLoopBuilder::with_user_event().build();

    let proxy = event_loop.create_proxy();

    let (task_sender, task_recv) = flume::unbounded();

    let handle = ExecutorHandle::new(proxy.clone(), task_sender);
    HANDLE.set(handle).expect("This cannot be happen");

    let handle = HANDLE.get().unwrap();

    unsafe {
        handle.spawn_unchecked(async move {
            main.await;
            proxy.send_event(ExecutorEvent::Exit(0)).unwrap();
        })
    }
    .detach();

    let mut executor = Executor { handle, task_recv };

    event_loop
        .run(move |event, target, control_flow| executor.on_event(event, target, control_flow));
}
