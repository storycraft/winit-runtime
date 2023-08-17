/*
 * Created on Thu Aug 10 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{
    marker::PhantomPinned,
    mem,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures_lite::Future;
use higher_kinded_types::ForLifetime;
use parking_lot::Mutex;
use pin_project::pinned_drop;

use pin_list::id::Unchecked;
use unique::Unique;

#[derive(Debug)]
pub struct EventSource<T: ForLifetime> {
    list: Mutex<PinList<T>>,
}

impl<T: ForLifetime> EventSource<T> {
    pub fn new() -> Self {
        Self {
            list: Mutex::new(PinList::new(unsafe { Unchecked::new() })),
        }
    }

    pub fn emit<'a>(&self, mut event: T::Of<'a>) {
        let mut list = self.list.lock();

        let mut cursor = list.cursor_front_mut();
        while let Some(node) = cursor.protected_mut() {
            // SAFETY: Closure is pinned and the pointer valid
            if unsafe { node.poll(&mut event) } {
                node.waker.wake_by_ref();
            }

            cursor.move_next();
        }
    }

    pub fn on<F: FnMut(&mut T::Of<'_>) -> Option<()> + Send>(
        &self,
        listener: F,
    ) -> EventFnFuture<F, T> {
        EventFnFuture {
            source: self,
            listener,
            node: pin_list::Node::new(),
            _pinned: PhantomPinned,
        }
    }

    pub async fn once<F: FnMut(&mut T::Of<'_>) -> Option<R> + Send, R: Send>(
        &self,
        mut listener: F,
    ) -> R {
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

type NodeTypes<T> = dyn pin_list::Types<
    Id = pin_list::id::Unchecked,
    Protected = ListenerItem<T>,
    Unprotected = (),
    Removed = (),
>;

type PinList<T> = pin_list::PinList<NodeTypes<T>>;

type Node<T> = pin_list::Node<NodeTypes<T>>;

#[pin_project::pin_project(PinnedDrop)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct EventFnFuture<'a, F, T: ForLifetime> {
    source: &'a EventSource<T>,

    listener: F,

    #[pin]
    node: Node<T>,

    _pinned: PhantomPinned,
}

impl<'a, T: ForLifetime, F: FnMut(&mut T::Of<'_>) -> Option<()> + Send> Future
    for EventFnFuture<'a, F, T>
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        let mut list = this.source.list.lock();

        if this.node.as_mut().initialized_mut().is_none() {
            list.push_back(
                this.node.as_mut(),
                ListenerItem::new(cx.waker().clone(), this.listener),
                (),
            );
        }

        let node = {
            let initialized = this.node.initialized_mut().unwrap();
            initialized.protected_mut(&mut list).unwrap()
        };

        if node.done {
            return Poll::Ready(());
        }

        if !node.waker.will_wake(cx.waker()) {
            node.update_waker(cx.waker().clone());
        }

        Poll::Pending
    }
}

#[pinned_drop]
impl<F, T: ForLifetime> PinnedDrop for EventFnFuture<'_, F, T> {
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

struct ListenerItem<T: ForLifetime> {
    done: bool,
    waker: Waker,
    closure_ptr: Unique<dyn FnMut(&mut T::Of<'_>) -> Option<()> + Send>,
}

impl<T: ForLifetime> ListenerItem<T> {
    pub fn new<'a>(
        waker: Waker,
        closure_ptr: &'a mut (dyn FnMut(&mut T::Of<'_>) -> Option<()> + Send),
    ) -> Self
    where
        T: 'a,
    {
        Self {
            done: false,
            waker,

            // Safety: See ListenerItem::poll for safety requirement
            closure_ptr: unsafe { mem::transmute::<_, Unique<_>>(Unique::from(closure_ptr)) },
        }
    }

    pub fn update_waker(&mut self, waker: Waker) {
        self.waker = waker;
    }

    /// # Safety
    /// Calling this method is only safe if pointer of closure is valid
    pub unsafe fn poll(&mut self, event: &mut T::Of<'_>) -> bool {
        if self.closure_ptr.as_mut()(event).is_some() && !self.done {
            self.done = true;
        }

        self.done
    }
}
