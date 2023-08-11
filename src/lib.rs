/*
 * Created on Sat Aug 05 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use event::Subscription;
use executor::executor_handle;
use futures_lite::Future;
use task::Task;

pub mod event;
pub mod executor;
pub mod timer;
pub mod window;

pub use async_task as task;

pub fn spawn_ui_task<Fut>(fut: Fut) -> Task<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    executor_handle().spawn(fut)
}

pub fn suspended() -> Subscription<'static> {
    executor_handle().suspended()
}

pub async fn exit(code: i32) -> ! {
    executor_handle().exit(code).await
}

#[inline(always)]
pub fn run(fut: impl Future<Output = ()> + 'static) -> ! {
    executor::run(fut)
}
