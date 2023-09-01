/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use instant::Duration;
use wm::{spawn_ui_task, timer::wait};

fn main() {
    wm::run(async {
        let task1 = spawn_ui_task(async move {
            println!("Sub task1 started");

            // wait for 2 secs (Async timer implemented on winit eventloop)
            wait(Duration::from_secs(2)).await;

            println!("Sub task1 done");
        });

        // Spawn another task which run on eventloop concurrently
        let task2 = spawn_ui_task(async move {
            println!("Sub task2 started");

            // wait for 1 sec
            wait(Duration::from_secs(1)).await;

            println!("Sub task2 done");
        });

        task1.await;
        task2.await;
    })
    .unwrap();
}
