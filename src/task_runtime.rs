use core::pin::Pin;
use core::ops::{Add, AddAssign};
use alloc::{
    prelude::*,
    sync::Arc,
    task::{Wake, local_waker_from_nonlocal},
    collections::{BTreeMap, BTreeSet},
};
use futures::{
    prelude::*,
    future::{FutureObj, LocalFutureObj, UnsafeFutureObj},
    task::{Poll, Spawn, LocalSpawn, SpawnError},
    channel::mpsc,
};
use mpsc_queue::{Queue, PopResult};

pub struct Executor {
    tasks: BTreeMap<TaskId, Pin<Box<LocalFutureObj<'static, ()>>>>,
    woken_tasks: Arc<Queue<TaskId>>,
    next_task_id: TaskId,
    idle_task: Option<Pin<Box<LocalFutureObj<'static, !>>>>,
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
            tasks: BTreeMap::new(),
            woken_tasks: Arc::new(Queue::new()),
            next_task_id: TaskId(0),
            idle_task: None,
        }
    }

    fn add_task(&mut self, task: Pin<Box<LocalFutureObj<'static, ()>>>) {
        let id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(id, task);
        self.woken_tasks.push(id);
    }

    pub fn set_idle_task<Fut>(&mut self, future: Fut)
    where
        Fut: Future<Output = !> + 'static,
    {
        let future_obj = Box::pinned(LocalFutureObj::new(Box::new(future)));
        self.idle_task = Some(future_obj);
    }

    pub fn run(&mut self) {
        match self.woken_tasks.pop() {
            PopResult::Data(task_id) => {
                let waker = MyWaker {
                    task_id,
                    woken_tasks: self.woken_tasks.clone(),
                };
                let poll_result = {
                    let task = self.tasks.get_mut(&task_id).expect(&format!("task with id {:?} not found", task_id));
                    task.as_mut().poll(&local_waker_from_nonlocal(Arc::new(waker)))
                };
                if poll_result.is_ready() {
                    self.tasks.remove(&task_id).expect(&format!("Task {:?} not found", task_id));
                }
            }
            PopResult::Empty => {}
            PopResult::Inconsistent => panic!("woken_tasks queue is inconsistent"),
        }
        if let Some(ref mut idle_task) = self.idle_task {
            idle_task.as_mut().poll(&local_waker_from_nonlocal(Arc::new(NoOpWaker)));
        };
    }
}

struct MyWaker {
    task_id: TaskId,
    woken_tasks: Arc<Queue<TaskId>>,
}

impl Wake for MyWaker {
    fn wake(arc_self: &Arc<Self>) {
        arc_self.woken_tasks.push(arc_self.task_id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl Add<u64> for TaskId {
    type Output = TaskId;

    fn add(self, other: u64) -> TaskId {
        TaskId(self.0 + other)
    }
}

impl AddAssign<u64> for TaskId {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

struct NoOpWaker;

impl Wake for NoOpWaker {
    fn wake(_arc_self: &Arc<Self>) {
    }
}
