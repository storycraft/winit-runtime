/*
 * Created on Mon Aug 07 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use instant::Duration;
use wm::{executor::executor_handle, spawn_ui_task, timer::wait, window::create_window};

fn main() {
    wm::run(async {
        println!("Hello async winit world!");

        // wait for one resume event to be done
        executor_handle()
            .resumed
            .once(|_| {
                // Run on Resume event is being called.
                println!("Called on resume!");

                Some(())
            })
            .await;
        println!("resume event done");

        let _window = create_window().unwrap();

        // Spawn another task which run on eventloop concurrently
        spawn_ui_task(async move {
            println!("Sub task1 started");

            // wait for 2 secs (Async timer implemented on winit eventloop)
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

        loop {
            // Wait for next device events. The closure is always FnMut since there can be multiple events before waking the task.
            executor_handle()
                .device
                .on(|(_, event)| {
                    dbg!(event);

                    Some(())
                })
                .await;

            println!("loop");
        }
    })
}
