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
use parking_lot::Mutex;
use pin_project::pinned_drop;
use tokio::sync::{futures::Notified, Notify};

use pin_list::{id::Unchecked, PinList};

#[derive(Debug)]
pub struct AsyncEventTarget(Notify);

impl AsyncEventTarget {
    pub const fn new() -> Self {
        Self(Notify::const_new())
    }

    pub fn subscribe(&self) -> Subscription {
        Subscription(self.0.notified())
    }

    pub fn dispatch(&self) {
        self.0.notify_waiters();
    }
}

impl Default for AsyncEventTarget {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[pin_project::pin_project]
pub struct Subscription<'a>(#[pin] Notified<'a>);

impl Future for Subscription<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.poll(cx)
    }
}

#[derive(Debug)]
pub struct EventSource<E: ?Sized> {
    list: Mutex<PinList<ListenerData<E>>>,
}

impl<E: ?Sized> EventSource<E> {
    pub fn new() -> Self {
        Self {
            list: Mutex::new(PinList::new(unsafe { Unchecked::new() })),
        }
    }

    pub fn dispatch(&self, event: &mut E) {
        let mut list = self.list.lock();

        let mut cursor = list.cursor_front_mut();
        while let Some((ref waker, ref mut data)) = cursor.protected_mut() {
            if (unsafe { data.as_mut() }).poll(event) {
                waker.wake_by_ref();
            }

            cursor.move_next();
        }
    }

    pub fn on<F: FnMut(&mut E) -> Option<()> + Send>(&self, listener: F) -> EventFnFuture<F, E> {
        EventFnFuture {
            source: self,
            data_sealed: Data {
                listener,
                done: false,
            },
            node: pin_list::Node::new(),
        }
    }

    pub async fn once<F: FnMut(&mut E) -> Option<T> + Send, T: Send>(&self, mut listener: F) -> T {
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

unsafe impl<E: ?Sized> Send for EventSource<E> {}
unsafe impl<E: ?Sized> Sync for EventSource<E> {}

type ListenerData<E> = dyn pin_list::Types<
    Id = pin_list::id::Unchecked,
    Protected = (Waker, NonNull<dyn PollData<E>>),
    Unprotected = (),
    Removed = (),
>;

#[pin_project::pin_project(PinnedDrop)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct EventFnFuture<'a, F, E: ?Sized> {
    source: &'a EventSource<E>,

    data_sealed: Data<F>,

    #[pin]
    node: pin_list::Node<ListenerData<E>>,
}

impl<'a, E: ?Sized, F: FnMut(&mut E) -> Option<()>> Future for EventFnFuture<'a, F, E> {
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
                        mem::transmute::<NonNull<dyn PollData<E>>, NonNull<dyn PollData<E>>>(
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
impl<F, E: ?Sized> PinnedDrop for EventFnFuture<'_, F, E> {
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

trait PollData<E: ?Sized> {
    fn poll(&mut self, event: &mut E) -> bool;
}

impl<E: ?Sized, F: FnMut(&mut E) -> Option<()>> PollData<E> for Data<F> {
    fn poll(&mut self, event: &mut E) -> bool {
        if (self.listener)(event).is_some() && !self.done {
            self.done = true;
        }

        self.done
    }
}
