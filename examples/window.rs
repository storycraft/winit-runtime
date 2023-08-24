/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use winit::{event::WindowEvent, window::Window};
use wm::{resumed, window};

fn main() {
    wm::run(async {
        // wait for next resume event
        let _window = resumed()
            .once(|target|
                // create window, on resume event
                Some(Window::new(target).unwrap())
            )
            .await;

        window().once(|(_, event, _)| {
            if let WindowEvent::CloseRequested = event {
                Some(())
            } else {
                None
            }
        }).await;
    })
}
