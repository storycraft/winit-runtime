/*
 * Created on Thu Aug 10 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

use futures_intrusive::timer::{Clock, Timer, TimerService};

pub use futures_intrusive::timer::TimerFuture;
use instant::Duration;

use crate::executor::executor_handle;

#[derive(Debug)]
pub(crate) struct ExecutorTimer {
    service: TimerService,
    next_expiration: AtomicU64,
}

impl ExecutorTimer {
    pub fn new() -> Self {
        struct InstantClock;

        impl Clock for InstantClock {
            fn now(&self) -> u64 {
                instant::now() as u64
            }
        }

        Self {
            service: TimerService::new(&InstantClock),
            next_expiration: AtomicU64::new(0),
        }
    }

    pub fn update_next(&self) -> UpdateState {
        let next = self.next_expiration.load(Ordering::Acquire);
        if next == 0 {
            return UpdateState::None;
        }

        let now = instant::now() as u64;

        if next <= now {
            self.service.check_expirations();
            self.next_expiration.store(
                self.service.next_expiration().unwrap_or(0),
                Ordering::Release,
            );

            UpdateState::Triggered
        } else {
            UpdateState::WaitTimeout(NonZeroU64::new(next - now).unwrap())
        }
    }

    pub fn delay(&self, delay: Duration) -> TimerFuture {
        self.deadline(instant::now() as u64 + delay.as_millis() as u64)
    }

    pub fn deadline(&self, timestamp: u64) -> TimerFuture {
        let future = self.service.deadline(timestamp);

        let _ = self
            .next_expiration
            .fetch_update(Ordering::Release, Ordering::Acquire, |next| {
                if next == 0 || next > timestamp {
                    Some(timestamp)
                } else {
                    None
                }
            });

        future
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum UpdateState {
    None,
    Triggered,
    WaitTimeout(NonZeroU64),
}

/// Create Future waiting for given duration
pub fn wait(delay: Duration) -> TimerFuture<'static> {
    executor_handle().wait(delay)
}

/// Create Future waiting for given timestamp
pub fn wait_deadline(timestamp: u64) -> TimerFuture<'static> {
    executor_handle().wait_deadline(timestamp)
}
