/*
 * Created on Sat Aug 05 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

//! # wm: Async runtime over winit's eventloop
//! 
//! ## Features
//! 1. Alloc free async timer
//! 2. Zero cost event dispatching
//! 3. Spawn ui tasks anywhere. Tasks run in eventloop's thread concurrently

use event::EventSource;
use executor::{executor_handle, with_eventloop_target};
use futures_lite::Future;
use higher_kinded_types::ForLt;
use task::Task;

pub mod event;
pub mod executor;
pub mod timer;

pub use async_task as task;
use winit::{window::{WindowId, WindowBuilder, Window}, event::{WindowEvent, DeviceId, DeviceEvent}, error::OsError};

pub fn spawn_ui_task<Fut>(fut: Fut) -> Task<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    executor_handle().spawn(fut)
}

pub async fn exit(code: i32) -> ! {
    executor_handle().exit(code).await
}

pub fn window() -> &'static EventSource<ForLt!((WindowId, WindowEvent<'_>))> {
    &executor_handle().window
}

pub fn device() -> &'static EventSource<ForLt!((DeviceId, DeviceEvent))> {
    &executor_handle().device
}

pub fn resumed() -> &'static EventSource<ForLt!(())> {
    &executor_handle().resumed
}

pub fn suspended() -> &'static EventSource<ForLt!(())> {
    &executor_handle().suspended
}

pub fn redraw_requested() -> &'static EventSource<ForLt!(WindowId)> {
    &executor_handle().redraw_requested
}

pub fn build_window(builder: WindowBuilder) -> Result<Window, OsError> {
    with_eventloop_target(move |target| builder.build(target))
}

#[inline]
pub fn create_window() -> Result<Window, OsError> {
    build_window(WindowBuilder::new())
}

#[inline(always)]
pub fn run(fut: impl Future<Output = ()> + 'static) -> ! {
    executor::run(fut)
}
