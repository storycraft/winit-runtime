/*
 * Created on Mon Aug 07 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

//! Implementation of winit Executor

pub mod event;
pub mod handle;

use std::sync::OnceLock;

use async_task::Task;
use event_source::emit;
use futures_lite::Future;
use instant::Duration;
use scoped_tls_hkt::scoped_thread_local;
use winit::{
    error::EventLoopError,
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget},
};

use crate::{device, resumed, suspended, timer::UpdateState, window};

use self::{event::ExecutorEvent, handle::ExecutorHandle};

pub type EventLoopTarget = EventLoopWindowTarget<ExecutorEvent>;

static HANDLE: OnceLock<ExecutorHandle> = OnceLock::new();

/// Get current [`ExecutorHandle`]
///
/// There can be only one [`ExecutorHandle`] and will panic if executor did not start.
pub fn executor_handle() -> &'static ExecutorHandle {
    HANDLE.get().expect("Executor is not started")
}

scoped_thread_local!(static EL_TARGET: EventLoopTarget);

/// Run closure using current [`EventLoopTarget`]
///
/// Will panic if it called on outside of runtime thread
pub fn with_eventloop_target<R>(func: impl FnOnce(&EventLoopTarget) -> R) -> R {
    EL_TARGET.with(func)
}

#[derive(Debug)]
struct Executor {
    _main: Task<()>,
    handle: &'static ExecutorHandle,
}

impl Executor {
    fn on_event(&mut self, event: Event<ExecutorEvent>, target: &EventLoopTarget) {
        EL_TARGET.set(target, move || match event {
            Event::UserEvent(ExecutorEvent::Wake) => {}

            Event::UserEvent(ExecutorEvent::PollTask(runnable)) => {
                runnable.run();
            }

            Event::UserEvent(ExecutorEvent::Exit) => target.exit(),

            Event::DeviceEvent { device_id, event } => {
                emit!(device(), (device_id, &event));
            }

            Event::WindowEvent {
                window_id,
                mut event,
            } => {
                emit!(window(), (window_id, &mut event));
            }

            Event::Resumed => {
                emit!(resumed(), ());
            }

            Event::Suspended => {
                emit!(suspended(), ());
            }

            Event::AboutToWait => {
                if let UpdateState::WaitTimeout(next_delay) = self.handle.timer.update_next() {
                    target.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(
                        next_delay.get(),
                    )));
                } else if target.control_flow() == ControlFlow::Poll {
                    target.set_control_flow(ControlFlow::Wait);
                }
            }

            _ => {}
        });
    }
}

/// Entrypoint for runtime
pub fn run(main: impl Future<Output = ()>) -> Result<(), EventLoopError> {
    let event_loop = EventLoopBuilder::with_user_event().build()?;

    let handle = {
        if HANDLE.set(ExecutorHandle::new(&event_loop)).is_err() {
            panic!("This cannot be happen");
        }

        HANDLE.get().unwrap()
    };

    let (runnable, task) = {
        let proxy = event_loop.create_proxy();

        let main = async move {
            main.await;
            let _ = proxy.send_event(ExecutorEvent::Exit);
        };

        // SAFETY: EventLoop created on same function, closure does not need to be Send and task and references to Future outlive event loop
        unsafe { handle.spawn_raw_unchecked(main) }
    };

    let mut executor = Executor {
        _main: task,
        handle,
    };

    EL_TARGET.set(&event_loop, move || runnable.run());

    event_loop.run(move |event, target| executor.on_event(event, target))
}
