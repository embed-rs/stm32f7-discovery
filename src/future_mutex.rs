//! Provides a non-blocking Mutex based on Futures.

use crate::mpsc_queue::{PopResult, Queue};
use core::task::Waker;
use core::{future::Future, mem, pin::Pin};
use futures::task::Poll;
use spin::Mutex;

/// A Mutex that yields instead of blocking.
pub struct FutureMutex<T> {
    mutex: Mutex<T>,
    waker_queue: Queue<Waker>,
}

impl<T> FutureMutex<T> {
    /// Creates a new Mutex wrapping the given data.
    pub fn new(user_data: T) -> Self {
        FutureMutex {
            mutex: Mutex::new(user_data),
            waker_queue: Queue::new(),
        }
    }
}

impl<T> FutureMutex<T> {
    /// Lock the mutex and execute the passed closure on the data.
    pub fn with<'a, R, F>(&'a self, f: F) -> impl Future<Output = R> + 'a
    where
        F: FnOnce(&mut T) -> R + Unpin + 'a,
        R: 'a,
    {
        FutureMutexResult {
            mutex: &self.mutex,
            f: Some(f),
            waker_queue: &self.waker_queue,
        }
    }
}

#[must_use = "futures do nothing unless polled"]
struct FutureMutexResult<'a, T, R, F>
where
    F: FnOnce(&mut T) -> R,
{
    mutex: &'a Mutex<T>,
    f: Option<F>,
    waker_queue: &'a Queue<Waker>,
}

impl<'a, T, R, F> Future for FutureMutexResult<'a, T, R, F>
where
    F: FnOnce(&mut T) -> R + Unpin,
{
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, lw: &Waker) -> Poll<Self::Output> {
        match self.mutex.try_lock() {
            None => {
                self.waker_queue.push(lw.clone());
                Poll::Pending
            }
            Some(mut guard) => {
                let f = self.f.take().unwrap();
                let ret = f(&mut guard);
                loop {
                    match self.waker_queue.pop() {
                        PopResult::Data(waker) => {
                            waker.wake();
                        }
                        PopResult::Empty => break,
                        PopResult::Inconsistent => panic!("woken_tasks queue is inconsistent"),
                    }
                }
                mem::drop(guard);
                Poll::Ready(ret)
            }
        }
    }
}
