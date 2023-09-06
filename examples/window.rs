/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use winit::event::WindowEvent;
use winit_runtime::{create_window, resumed, window};

fn main() {
    winit_runtime::run(async {
        // wait for next resume event
        let _window = resumed()
            .once(|_|
                // create window, on resume event
                Some(create_window().unwrap()))
            .await;

        window()
            .once(|(_, event)| {
                if let WindowEvent::CloseRequested = event {
                    Some(())
                } else {
                    None
                }
            })
            .await;
    })
    .unwrap();
}
