/*
 * Created on Mon Aug 07 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use instant::Duration;
use wm::{spawn_ui_task, timer::wait, window::create_window, resumed};

fn main() {
    wm::run(async {
        println!("Hello async winit world!");

        resumed().await;

        let _window = create_window().unwrap();

        spawn_ui_task(async move {
            println!("Sub task1 started");

            wait(Duration::from_secs(2)).await;

            println!("Sub task1 done");
        })
        .detach();

        spawn_ui_task(async move {
            println!("Sub task2 started");

            wait(Duration::from_secs(1)).await;

            println!("Sub task2 done");
        })
        .detach();

        wait(Duration::from_secs(3)).await;
        println!("Main task done");
    })
}
