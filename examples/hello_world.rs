/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

fn main() {
    // Spawn winit eventloop and run main task
    winit_runtime::run(async {
        println!("Hello async winit world!");
    })
    .unwrap();
}
