/*
 * Created on Thu Aug 10 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{
    mem,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use futures_lite::Future;
use higher_kinded_types::ForLifetime;
use parking_lot::Mutex;
use pin_project::pinned_drop;

use pin_list::{id::Unchecked, PinList};

#[derive(Debug)]
pub struct EventSource<T> {
    list: Mutex<PinList<ListenerData<T>>>,
}

impl<T: ForLifetime> EventSource<T> {
    pub fn new() -> Self {
        Self {
            list: Mutex::new(PinList::new(unsafe { Unchecked::new() })),
        }
    }

    pub fn emit<'a>(&self, event: T::Of<'a>) where T::Of<'a>: Clone {
        let mut list = self.list.lock();

        let mut cursor = list.cursor_front_mut();
        while let Some((ref waker, ref mut data)) = cursor.protected_mut() {
            if (unsafe { data.as_mut() }).poll(event.clone()) {
                waker.wake_by_ref();
            }

            cursor.move_next();
        }
    }

    pub fn on<F: FnMut(T::Of<'_>) -> Option<()> + Send>(&self, listener: F) -> EventFnFuture<F, T> {
        EventFnFuture {
            source: self,
            data_sealed: Data {
                listener,
                done: false,
            },
            node: pin_list::Node::new(),
        }
    }

    pub async fn once<F: FnMut(T::Of<'_>) -> Option<R> + Send, R: Send>(&self, mut listener: F) -> R {
        let mut res = None;

        self.on(|event| {
            if res.is_some() {
                return None;
            }

            listener(event).map(|output| {
                res = Some(output);
            })
        })
        .await;

        res.unwrap()
    }
}

unsafe impl<T> Send for EventSource<T> {}
unsafe impl<T> Sync for EventSource<T> {}

type ListenerData<T> = dyn pin_list::Types<
    Id = pin_list::id::Unchecked,
    Protected = (Waker, NonNull<dyn PollData<T>>),
    Unprotected = (),
    Removed = (),
>;

#[pin_project::pin_project(PinnedDrop)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct EventFnFuture<'a, F, T> {
    source: &'a EventSource<T>,

    data_sealed: Data<F>,

    #[pin]
    node: pin_list::Node<ListenerData<T>>,
}

impl<'a, T: ForLifetime, F: FnMut(T::Of<'_>) -> Option<()>> Future for EventFnFuture<'a, F, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        let mut lock = this.source.list.lock();

        if this.data_sealed.done {
            Poll::Ready(())
        } else {
            if let Some(node) = this.node.as_mut().initialized_mut() {
                let (ref mut waker, _) = node.protected_mut(&mut lock).unwrap();

                if !waker.will_wake(cx.waker()) {
                    *waker = cx.waker().clone();
                }
            } else {
                lock.push_back(
                    this.node,
                    (cx.waker().clone(), unsafe {
                        mem::transmute::<NonNull<dyn PollData<T>>, NonNull<dyn PollData<T>>>(
                            NonNull::from(this.data_sealed),
                        )
                    }),
                    (),
                );
            }

            Poll::Pending
        }
    }
}

#[pinned_drop]
impl<F, T> PinnedDrop for EventFnFuture<'_, F, T> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        let node = match this.node.initialized_mut() {
            Some(initialized) => initialized,
            None => return,
        };

        let mut list = this.source.list.lock();

        let _ = node.reset(&mut list);
    }
}

struct Data<F> {
    listener: F,
    done: bool,
}


trait PollData<T: ForLifetime> {
    fn poll(&mut self, event: T::Of<'_>) -> bool;
}

impl<T: ForLifetime, F: FnMut(T::Of<'_>) -> Option<()>> PollData<T> for Data<F> {
    fn poll(&mut self, event: T::Of<'_>) -> bool {
        if (self.listener)(event).is_some() && !self.done {
            self.done = true;
        }

        self.done
    }
}
