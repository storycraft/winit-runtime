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

        executor_handle()
            .resumed
            .on(|_| {
                println!("Called on resume!");
                Some(())
            })
            .await;
        println!("resume event done");

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

        loop {
            executor_handle()
                .device
                .on(|(_, event)| {
                    dbg!(event);

                    Some(())
                })
                .await;

            println!("loop");
        }

        wait(Duration::from_secs(3)).await;
        println!("Main task done");
    })
}
