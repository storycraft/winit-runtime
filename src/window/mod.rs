/*
 * Created on Sat Aug 05 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use crate::executor::with_eventloop_target;

use winit::{
    error::OsError,
    window::{Window, WindowBuilder},
};

pub fn build_window(builder: WindowBuilder) -> Result<Window, OsError> {
    with_eventloop_target(move |target| builder.build(target))
}

pub fn create_window() -> Result<Window, OsError> {
    build_window(WindowBuilder::new())
}
