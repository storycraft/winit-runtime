/*
 * Created on Mon Aug 07 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

pub mod event;
pub mod handle;

use std::sync::OnceLock;

use futures_lite::Future;
use instant::Duration;
use scoped_tls_hkt::scoped_thread_local;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget},
};

use crate::timer;

use self::{event::ExecutorEvent, handle::ExecutorHandle};

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
                EL_TARGET.set(target, move || {
                    runnable.run();
                });
            }

            Event::NewEvents(_) => {
                self.handle.timer.check_expirations();
            }

            Event::UserEvent(ExecutorEvent::Exit(code)) => {
                *control_flow = ControlFlow::ExitWithCode(code);
            }

            Event::MainEventsCleared => {}

            Event::RedrawRequested(id) => {
                self.handle.redraw_requested.emit((id, target));
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

            Event::DeviceEvent { device_id, event } => {
                self.handle.device.emit((device_id, event, target));
            }

            Event::WindowEvent {
                window_id,
                event,
            } => {
                self.handle
                    .window
                    .emit((window_id, event, target));
            }

            Event::Resumed => {
                self.handle.resumed.emit(target);
            }

            Event::Suspended => {
                self.handle.suspended.emit(target);
            }

            _ => {}
        }
    }
}

pub fn run(main: impl Future<Output = ()> + 'static) -> ! {
    let event_loop = EventLoopBuilder::with_user_event().build();

    let proxy = event_loop.create_proxy();

    if HANDLE
        .set(ExecutorHandle::new(proxy.clone(), timer::create_service()))
        .is_err()
    {
        panic!("This cannot be happen");
    }

    let handle = HANDLE.get().unwrap();

    let mut executor = Executor { handle };

    // SAFETY: EventLoop created on same function, closure does not need to be Send
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
