//! An experimental runtime for an async-await style task system.

use crate::mpsc_queue::{PopResult, Queue};
use alloc::{
    collections::BTreeMap,
    prelude::v1::*,
    sync::Arc,
};
use core::ops::{Add, AddAssign};
use core::pin::Pin;
use futures::{
    future::{FutureObj, LocalFutureObj},
    prelude::*,
    task::{LocalSpawn, Poll, Spawn, SpawnError, Waker, RawWaker, RawWakerVTable},
};

pub mod mpsc;

/// An executor that schedules tasks round-robin, and executes an idle_task
/// if no task is ready to execute.
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
        self.add_task(Box::pin(future));
        Ok(())
    }
}

impl Executor {
    /// Creates a new executor.
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

    /// Sets the specified task as idle task.
    ///
    /// It will be polled whenever there is no ready-to-run task in the queue.
    pub fn set_idle_task<Fut>(&mut self, future: Fut)
    where
        Fut: Future<Output = !> + 'static,
    {
        let future_obj = Box::pin(LocalFutureObj::new(Box::new(future)));
        self.idle_task = Some(future_obj);
    }

    /// Poll all tasks that are ready to run, until no ready tasks exist. Then poll the idle task
    /// once and return.
    pub fn run(&mut self) {
        match self.woken_tasks.pop() {
            PopResult::Data(task_id) => {
                let waker = MyWaker {
                    task_id,
                    woken_tasks: self.woken_tasks.clone(),
                };
                let poll_result = {
                    let task = self.tasks.get_mut(&task_id).unwrap_or_else(|| panic!("task with id {:?} not found", task_id));
                    task.as_mut().poll(&waker.into_waker())
                };
                if poll_result.is_ready() {
                    self.tasks.remove(&task_id).unwrap_or_else(|| panic!("Task {:?} not found", task_id));
                }
            }
            PopResult::Empty => {}
            PopResult::Inconsistent => {} // println!("woken_tasks queue is inconsistent"),
        }
        if let Some(ref mut idle_task) = self.idle_task {
            let _ = idle_task
                .as_mut()
                .poll(&NoOpWaker.into_waker());
        };
    }
}

#[derive(Clone)]
struct MyWaker {
    task_id: TaskId,
    woken_tasks: Arc<Queue<TaskId>>,
}

const MY_WAKER_VTABLE: RawWakerVTable = unsafe { RawWakerVTable {
    drop: core::mem::transmute(MyWaker::waker_drop as fn(Box<MyWaker>)),
    wake: core::mem::transmute(MyWaker::wake as fn(&MyWaker)),
    clone: core::mem::transmute(MyWaker::waker_clone as fn(&MyWaker) -> RawWaker),
}};

impl MyWaker {
    fn into_raw_waker(self) -> RawWaker {
        RawWaker::new(Box::into_raw(Box::new(self)) as *const (), &MY_WAKER_VTABLE)
    }
    fn waker_drop(_: Box<Self>) {}
    fn waker_clone(&self) -> RawWaker {
        self.clone().into_raw_waker()
    }
    fn wake(&self) {
        self.woken_tasks.push(self.task_id);
    }
    fn into_waker(self) -> Waker {
        unsafe {
            Waker::new_unchecked(self.into_raw_waker())
        }
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

impl NoOpWaker {
    fn into_waker(self) -> Waker {
        unsafe {
            Waker::new_unchecked(self.into_raw_waker())
        }
    }
    fn into_raw_waker(self) -> RawWaker {
        RawWaker::new(
            &NoOpWaker as *const _ as *const (),
            &RawWakerVTable {
                drop: (|_| {}) as fn(*const ()),
                wake: (|_| {}) as fn(*const ()),
                clone: (|_| NoOpWaker.into_raw_waker()) as fn(*const ()) -> RawWaker,
            },
        )
    }
}

/// This stream can be used by tasks that want to run when the CPU is idle.
///
/// It works by alternately returning `Poll::Ready` and `Poll::Pending` from `poll_next`, starting
/// with `Poll::Pending`. When returning `Poll::Pending` it sends the Waker to the
/// `idle_waker_sink` (passed on construction). The idle task polls the other end of this sink and
/// wakes all received tasks when it runs.
// TODO is the behavior correct?
#[derive(Debug, Clone)]
pub struct IdleStream {
    idle: bool,
    idle_waker_sink: mpsc::UnboundedSender<Waker>,
}

impl IdleStream {
    /// Creates a new IdleStream with the passed sending end of an idle stream.
    ///
    /// The idle task should wake the tasks received from the receiving end
    /// of the idle stream, thereby waking the tasks on idle.
    pub fn new(idle_waker_sink: mpsc::UnboundedSender<Waker>) -> Self {
        IdleStream {
            idle_waker_sink,
            idle: false,
        }
    }
}

impl futures::prelude::Stream for IdleStream {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<()>> {
        let result = if self.idle {
            Poll::Ready(Some(()))
        } else {
            self.idle_waker_sink
                .unbounded_send(waker.clone())
                .expect("sending on idle channel failed");
            Poll::Pending
        };
        self.idle = !self.idle;
        result
    }
}
