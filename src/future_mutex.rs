use spin::{Mutex, MutexGuard};
use core::{
    future::Future,
    sync::atomic::AtomicBool,
    pin::{Pin, Unpin},
    mem,
};
use alloc::{
    prelude::*,
    sync::Arc,
    task::{Wake, LocalWaker, local_waker_from_nonlocal},
    collections::{BTreeMap, BTreeSet},
};
use futures::{
    prelude::*,
    future::{FutureObj, LocalFutureObj, UnsafeFutureObj},
    task::{Poll, Spawn, LocalSpawn, SpawnError},
    channel::mpsc,
};
use mpsc_queue::{Queue, PopResult};

pub struct FutureMutex<T> {
    mutex: Mutex<T>,
    waker_queue: Queue<LocalWaker>,
}

impl<T> FutureMutex<T> {
    pub fn new(user_data: T) -> Self {
        FutureMutex {
            mutex: Mutex::new(user_data),
            waker_queue: Queue::new(),
        }
    }
}

impl<T> FutureMutex<T> {
    pub fn with<'a, R, F>(&'a self, f: F) -> FutureMutexResult<'a, T, R, F> where F: FnOnce(&mut T) -> R + 'a, R: 'a {
        FutureMutexResult {
            mutex: &self.mutex,
            f: Some(f),
            waker_queue: &self.waker_queue,
        }
    }

    pub fn force_lock(&self) {
        println!("force lock");
        ::core::mem::forget(self.mutex.lock())
    }

    pub fn force_unlock(&self) {
        println!("force unlock");
        loop {
            match self.waker_queue.pop() {
                PopResult::Data(waker) => {
                    waker.wake();
                    println!("force unlock: wake");
                }
                PopResult::Empty => break,
                PopResult::Inconsistent => panic!("woken_tasks queue is inconsistent"),
            }
        }
        unsafe { self.mutex.force_unlock() }
        println!("force unlock done");
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct FutureMutexResult<'a, T, R, F> where F: FnOnce(&mut T) -> R {
    mutex: &'a Mutex<T>,
    f: Option<F>,
    waker_queue: &'a Queue<LocalWaker>,
}

impl<'a, T, R, F> Future for FutureMutexResult<'a, T, R, F> where F: FnOnce(&mut T) -> R + Unpin {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Self::Output> {
        match self.mutex.try_lock() {
            None => {
                self.waker_queue.push(lw.clone());
                Poll::Pending
            },
            Some(mut guard) => {
                let mut f = self.f.take().unwrap();
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
            },
        }
    }
}
