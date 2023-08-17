/*
 * Created on Wed Aug 16 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::pin::pin;

use futures_lite::{future::poll_fn, Future};
use higher_kinded_types::ForLt;
use wm::event::EventSource;

#[tokio::test]
async fn test_event_source() {
    let source: EventSource<ForLt!(())> = EventSource::new();

    let mut called = 0;

    {
        let listener = source.on(|_| {
            called += 1;
            Some(())
        });
        let mut listener = pin!(listener);

        poll_fn(|cx| {
            let res = listener.as_mut().poll(cx);
            source.emit(());

            res
        })
        .await;
    }

    assert_eq!(called, 2);
}
