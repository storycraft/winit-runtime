/*
 * Created on Sat Aug 05 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use event::EventSource;
use executor::executor_handle;
use futures_lite::Future;
use higher_kinded_types::ForLt;
use task::Task;

pub mod event;
pub mod executor;
pub mod timer;
pub mod window;

pub use async_task as task;
use winit::{window::WindowId, event::{WindowEvent, DeviceId, DeviceEvent}};

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

#[inline(always)]
pub fn run(fut: impl Future<Output = ()> + 'static) -> ! {
    executor::run(fut)
}
