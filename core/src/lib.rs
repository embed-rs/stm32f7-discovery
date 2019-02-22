#![feature(futures_api)]
#![feature(optin_builtin_traits)]
#![feature(generator_trait)]
#![feature(arbitrary_self_types)]
#![no_std]

pub use core::*;

pub mod future {
    pub use core::future::*;

    use core::{
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr,
        sync::atomic::{AtomicPtr, Ordering},
        task::{Waker, Poll},
    };

    /// Wrap a future in a generator.
    ///
    /// This function returns a `GenFuture` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Future` rather than `GenFuture<[closure.....]>`).
    pub fn from_generator<T: Generator<Yield = ()>>(x: T) -> impl Future<Output = T::Return> {
        GenFuture(x)
    }

    /// A wrapper around generators used to implement `Future` for `async`/`await` code.
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
    #[must_use = "futures do nothing unless polled"]
    struct GenFuture<T: Generator<Yield = ()>>(T);

    // We rely on the fact that async/await futures are immovable in order to create
    // self-referential borrows in the underlying generator.
    impl<T: Generator<Yield = ()>> !Unpin for GenFuture<T> {}

    impl<T: Generator<Yield = ()>> Future for GenFuture<T> {
        type Output = T::Return;
        fn poll(self: Pin<&mut Self>, lw: &Waker) -> Poll<Self::Output> {
            // Safe because we're !Unpin + !Drop mapping to a ?Unpin value
            let gen = unsafe { Pin::map_unchecked_mut(self, |s| &mut s.0) };
            set_task_waker(lw, || match gen.resume() {
                GeneratorState::Yielded(()) => Poll::Pending,
                GeneratorState::Complete(x) => Poll::Ready(x),
            })
        }
    }

    // FIXME: Should be thread local, but is currently a static since we only have a single thread
    static TLS_WAKER: AtomicPtr<Waker> = AtomicPtr::new(ptr::null_mut());

    struct SetOnDrop(*mut Waker);

    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            TLS_WAKER.store(self.0, Ordering::SeqCst);
        }
    }

    /// Sets the thread-local task context used by async/await futures.
    pub fn set_task_waker<F, R>(lw: &Waker, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let old_waker = TLS_WAKER.swap(lw as *const _ as *mut _, Ordering::SeqCst);
        let _reset_waker = SetOnDrop(old_waker);
        f()
    }

    /// Retrieves the thread-local task waker used by async/await futures.
    ///
    /// This function acquires exclusive access to the task waker.
    ///
    /// Panics if no waker has been set or if the waker has already been
    /// retrieved by a surrounding call to get_task_waker.
    pub fn get_task_waker<F, R>(f: F) -> R
    where
        F: FnOnce(&Waker) -> R,
    {
        // Clear the entry so that nested `get_task_waker` calls
        // will fail or set their own value.
        let waker_ptr = TLS_WAKER.swap(ptr::null_mut(), Ordering::SeqCst);
        let _reset_waker = SetOnDrop(waker_ptr);

        let waker_ptr = unsafe { waker_ptr.as_ref() }.expect("TLS Waker not set.");
        f(waker_ptr)
    }

    /// Polls a future in the current thread-local task waker.
    pub fn poll_with_tls_waker<F>(f: Pin<&mut F>) -> Poll<F::Output>
    where
        F: Future,
    {
        get_task_waker(|lw| F::poll(f, lw))
    }

    #[macro_export]
    macro_rules! r#await {
        ($e:expr) => {{
            let mut pinned = $e;
            loop {
                if let core::task::Poll::Ready(x) = $crate::future::poll_with_tls_waker(unsafe {
                    core::pin::Pin::new_unchecked(&mut pinned)
                }) {
                    break x;
                }
                // FIXME(cramertj) prior to stabilizing await, we have to ensure that this
                // can't be used to create a generator on stable via `|| await!()`.
                yield
            }
        }};
    }

}
