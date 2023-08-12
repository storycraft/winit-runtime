/*
 * Created on Sat Aug 05 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use crate::executor::with_eventloop_target;

use winit::{
    error::OsError,
    window::{Window as WinitWindow, WindowBuilder, WindowId},
};

pub fn build_window(builder: WindowBuilder) -> Result<WinitWindow, OsError> {
    with_eventloop_target(move |target| builder.build(target))
}

pub fn create_window() -> Result<WinitWindow, OsError> {
    build_window(WindowBuilder::new())
}

#[derive(Debug)]
pub struct Window {
    winit: WinitWindow,
}

impl Window {
    pub fn init(winit: WinitWindow) -> Self {
        Self { winit }
    }
}

#[derive(Debug)]
pub struct WindowEventTarget {
    id: WindowId,
}
