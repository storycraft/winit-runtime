/*
 * Created on Thu Aug 10 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{pin::Pin, task::{Context, Poll}};

use futures_lite::Future;
use tokio::sync::{Notify, futures::Notified};

#[derive(Debug)]
pub struct AsyncEventTarget(Notify);

impl AsyncEventTarget {
    pub const fn new() -> Self {
        Self(Notify::const_new())
    }

    pub fn subscribe(&self) -> Subscription {
        Subscription(self.0.notified())
    }

    pub fn dispatch(&self) {
        self.0.notify_waiters();
    }
}

impl Default for AsyncEventTarget {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[pin_project::pin_project]
pub struct Subscription<'a>(#[pin] Notified<'a>);

impl Future for Subscription<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.poll(cx)
    }
}
