use core::pin::Pin;
use alloc::{
    prelude::*,
    sync::Arc,
    task::{Wake, local_waker_from_nonlocal},
    collections::BTreeMap,
};
use futures::{
    prelude::*,
    future::{FutureObj, LocalFutureObj, UnsafeFutureObj},
    task::{Spawn, LocalSpawn, SpawnError},
};
use spin::Mutex;

pub struct Executor {
    ready_tasks: Vec<(u64, Pin<Box<LocalFutureObj<'static, ()>>>)>,
    waiting_tasks: BTreeMap<u64, Pin<Box<LocalFutureObj<'static, ()>>>>,
    woken_tasks: Arc<Mutex<Vec<u64>>>,
    next_task_id: u64,
}

impl Spawn for Executor {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        self.spawn_local_obj(future.into())
    }
}

impl LocalSpawn for Executor {
    fn spawn_local_obj(&mut self, future: LocalFutureObj<'static, ()>) -> Result<(), SpawnError> {
        self.add_task(Box::pinned(future));
        Ok(())
    }
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            ready_tasks: Vec::new(),
            waiting_tasks: BTreeMap::new(),
            woken_tasks: Arc::new(Mutex::new(Vec::new())),
            next_task_id: 0,
        }
    }

    fn add_task(&mut self, task: Pin<Box<LocalFutureObj<'static, ()>>>) {
        let id = self.next_task_id;
        self.next_task_id += 1;
        self.ready_tasks.push((id, task));
    }

    pub fn run(&mut self) {
        {
            let mut woken_tasks = self.woken_tasks.lock();
            for task_id in woken_tasks.drain(..) {
                if let Some(task) = self.waiting_tasks.remove(&task_id) {
                    self.ready_tasks.push((task_id, task));
                }
            }
        }
        for (task_id, mut task) in self.ready_tasks.drain(..) {
            let waker = MyWaker {
                task_id,
                woken_tasks: self.woken_tasks.clone(),
            };
            let poll_result = task.as_mut().poll(&local_waker_from_nonlocal(Arc::new(waker)));
            if poll_result.is_pending() {
                self.waiting_tasks.insert(task_id, task);
            }
        };
    }
}

struct MyWaker {
    task_id: u64,
    woken_tasks: Arc<Mutex<Vec<u64>>>,
}

impl Wake for MyWaker {
    fn wake(arc_self: &Arc<Self>) {
        arc_self.woken_tasks.lock().push(arc_self.task_id);
    }
}
