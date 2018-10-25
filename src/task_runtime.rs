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
        for task in &mut self.tasks {
            task.as_mut().poll(&local_waker_from_nonlocal(Arc::new(MyWaker)));
        }   
    }
}

struct MyWaker;

impl Wake for MyWaker {
    fn wake(arc_self: &Arc<Self>) {
    }
}
