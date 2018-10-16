use core::future::Future;
use core::ops::{Generator, GeneratorState};
use core::marker::Unpin;
use core::task::{Poll, LocalWaker};
use core::pin::Pin;
use core::cell::Cell;
use core::ptr::NonNull;
use spin::Mutex;
use alloc::task::{local_waker, Wake};
use alloc::sync::Arc;

/// Wrap a future in a generator.
///
/// This function returns a `GenFuture` underneath, but hides it in `impl Trait` to give
/// better error messages (`impl Future` rather than `GenFuture<[closure.....]>`).
pub fn from_generator<T: Generator<Yield = ()>>(x: T) -> impl Future<Output = T::Return> {
    GenFuture(x)
}

/// A wrapper around generators used to implement `Future` for `async`/`await` code.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct GenFuture<T: Generator<Yield = ()>>(T);

// We rely on the fact that async/await futures are immovable in order to create
// self-referential borrows in the underlying generator.
impl<T: Generator<Yield = ()>> !Unpin for GenFuture<T> {}

impl<T: Generator<Yield = ()>> Future for GenFuture<T> {
    type Output = T::Return;
    fn poll(self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Self::Output> {
        match unsafe { Pin::get_mut_unchecked(self).0.resume() } {
            GeneratorState::Yielded(()) => Poll::Pending,
            GeneratorState::Complete(x) => Poll::Ready(x),
        }
    }
}

/// Polls a future in the current thread-local task waker.
pub fn poll_with_tls_waker<F>(f: Pin<&mut F>) -> Poll<F::Output>
where
    F: Future
{
    F::poll(f, unsafe {&local_waker(Arc::new(MyWaker(45)))})
}


#[macro_export]
macro_rules! await {
    ($e:expr) => { {
        let mut pinned = $e;
        loop {
            if let core::task::Poll::Ready(x) =
                $crate::future_runtime::poll_with_tls_waker(unsafe {
                    core::pin::Pin::new_unchecked(&mut pinned)
                })
            {
                break x;
            }
            // FIXME(cramertj) prior to stabilizing await, we have to ensure that this
            // can't be used to create a generator on stable via `|| await!()`.
            yield
        }
    } }
}




struct MyWaker(&'static AtomicBool);

impl Wake for MyWaker {
    fn wake(arc_self: &Arc<Self>) {
    }
}