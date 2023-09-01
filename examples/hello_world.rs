/*
 * Created on Thu Aug 17 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

fn main() {
    // Spawn winit eventloop and run main task
    wm::block_on(async {
        println!("Hello async winit world!");
    })
    .unwrap();
}
