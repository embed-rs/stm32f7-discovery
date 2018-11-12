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

pub struct Executor {
    tasks: BTreeMap<TaskId, Pin<Box<LocalFutureObj<'static, ()>>>>,
    ready_tasks: Vec<TaskId>,
    woken_tasks: mpsc::UnboundedReceiver<TaskId>,
    woken_tasks_sender: mpsc::UnboundedSender<TaskId>,
    next_task_id: TaskId,
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
        let (sender, receiver) = mpsc::unbounded();
        Executor {
            tasks: BTreeMap::new(),
            ready_tasks: Vec::new(),
            woken_tasks: receiver,
            woken_tasks_sender: sender,
            next_task_id: TaskId(0),
        }
    }

    fn add_task(&mut self, task: Pin<Box<LocalFutureObj<'static, ()>>>) {
        let id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(id, task);
        self.ready_tasks.push(id);
    }

    pub fn run(&mut self) {
        while let Ok(task_id) = self.woken_tasks.try_next() {
            let task_id = task_id.expect("woken_tasks stream has terminated");
            self.ready_tasks.push(task_id);
        }
        for task_id in self.ready_tasks.drain(..) {
            let waker = MyWaker {
                task_id,
                woken_tasks: self.woken_tasks_sender.clone(),
            };
            let poll_result = {
                let task = self.tasks.get_mut(&task_id).expect(&format!("task with id {:?} not found", task_id));
                task.as_mut().poll(&local_waker_from_nonlocal(Arc::new(waker)))
            };
            if poll_result.is_ready() {
                self.tasks.remove(&task_id).expect(&format!("Task {:?} not found", task_id));
            }
        };
    }
}

struct MyWaker {
    task_id: TaskId,
    woken_tasks: mpsc::UnboundedSender<TaskId>,
}

impl Wake for MyWaker {
    fn wake(arc_self: &Arc<Self>) {
        arc_self.woken_tasks.unbounded_send(arc_self.task_id);
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