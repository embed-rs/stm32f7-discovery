use core::pin::Pin;
use alloc::{
    prelude::*,
    sync::Arc,
    task::{Wake, local_waker_from_nonlocal},
};
use futures::{
    prelude::*,
    future::{FutureObj, LocalFutureObj, UnsafeFutureObj},
    task::{Spawn, LocalSpawn, SpawnError},
};

pub struct Executor {
    tasks: Vec<Pin<Box<LocalFutureObj<'static, ()>>>>,
}

impl Spawn for Executor {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        self.tasks.push(Box::pinned(future.into()));
        Ok(())
    }
}

impl LocalSpawn for Executor {
    fn spawn_local_obj(&mut self, future: LocalFutureObj<'static, ()>) -> Result<(), SpawnError> {
        self.tasks.push(Box::pinned(future));
        Ok(())
    }
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        self.tasks.drain_filter(|task| {
            let poll_result = task.as_mut().poll(&local_waker_from_nonlocal(Arc::new(MyWaker::new())));
            poll_result.is_ready()
        });
    }
}

struct MyWaker(usize);

impl MyWaker {
    fn new() -> Self {
        use core::sync::atomic::{AtomicUsize, Ordering};

        static INIT_COUNTER: AtomicUsize = AtomicUsize::new(0);

        MyWaker(INIT_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Wake for MyWaker {
    fn wake(arc_self: &Arc<Self>) {
    }
}
