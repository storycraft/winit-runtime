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

use executor::{executor_handle, with_eventloop_target};
use futures_lite::Future;
use task::Task;

pub mod event;
pub mod executor;
pub mod timer;

pub use async_task as task;
use winit::{
    error::{EventLoopError, OsError},
    event::{DeviceEvent, DeviceId, WindowEvent},
    window::{Window, WindowBuilder, WindowId},
};

#[inline]
pub fn spawn_ui_task<Fut>(fut: Fut) -> Task<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    executor_handle().spawn(fut)
}

#[inline]
pub fn spawn_local_ui_task<Fut>(fut: Fut) -> Task<Fut::Output>
where
    Fut: Future + 'static,
    Fut::Output: 'static,
{
    executor_handle().spawn_local(fut)
}

#[inline]
pub async fn exit(code: i32) -> ! {
    executor_handle().exit(code).await
}

macro_rules! define_event {
    (pub $name: ident: $ty: tt) => {
        pub fn $name() -> &'static event::EventSource<higher_kinded_types::ForLt!($ty)> {
            static SOURCE: event::EventSource<higher_kinded_types::ForLt!($ty)> =
                event::EventSource::new();

            &SOURCE
        }
    };
}

define_event!(pub window: (WindowId, WindowEvent));

define_event!(pub device: (DeviceId, DeviceEvent));

define_event!(pub resumed: ());

define_event!(pub suspended: ());

define_event!(pub redraw_requested: WindowId);

pub fn build_window(builder: WindowBuilder) -> Result<Window, OsError> {
    with_eventloop_target(move |target| builder.build(target))
}

#[inline]
pub fn create_window() -> Result<Window, OsError> {
    build_window(WindowBuilder::new())
}

#[inline(always)]
pub fn block_on<F: Future>(fut: F) -> Result<Option<F::Output>, EventLoopError> {
    executor::block_on(fut)
}
