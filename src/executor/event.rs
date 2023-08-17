/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use async_task::Runnable;

#[derive(Debug)]
#[non_exhaustive]
pub enum ExecutorEvent {
    PollTask(Runnable),
    TimerAdded,
    Exit(i32),
}
