/*
 * Created on Thu Aug 10 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use futures_intrusive::timer::{Clock, TimerService};

pub use futures_intrusive::timer::TimerFuture;
use instant::Duration;

use crate::executor::executor_handle;

pub(crate) fn create_service() -> TimerService {
    struct InstantClock;

    impl Clock for InstantClock {
        fn now(&self) -> u64 {
            instant::now() as u64
        }
    }

    TimerService::new(&InstantClock)
}

pub fn wait(delay: Duration) -> TimerFuture<'static> {
    executor_handle().wait(delay)
}

pub fn wait_deadline(timestamp: u64) -> TimerFuture<'static> {
    executor_handle().wait_deadline(timestamp)
}
