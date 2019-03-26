//! Mutex for interrupt synchronization.
//!
//! The mutex uses the `primask` core register of the cortex m processor
//! to disable interrupts and synchronize the access to shared variables.
//!
//! Since the access to the data is synchronized (no interrupt can preempt
//! the current code in the critical section) the mutex implements `Send` and `Sync` when
//! the synchronized data implements `Send`.

use core::cell::UnsafeCell;
use cortex_m::interrupt;
use cortex_m::register::primask;

/// Mutex that uses the `primask` core register from the cortem m processor to disable
/// interrupts before the critical section and enables interrupts again, when interrupts
/// were enabled before entering the critical section.
///
/// Since the access to the data is synchronized (no interrupt can preempt
/// the current code in the critical section) the mutex implements `Send` and `Sync` when
/// the synchronized data implements `Send`.
#[derive(Debug)]
pub struct PrimaskMutex<T> {
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for PrimaskMutex<T> {}
unsafe impl<T: Send> Sync for PrimaskMutex<T> {}

impl<T> PrimaskMutex<T> {
    /// Takes ownership over the data and return a new mutex
    ///
    /// # Examples
    /// ```
    /// let x = 5;
    /// let mutex = PrimaskMutex::new(x);
    /// ```
    pub fn new(data: T) -> PrimaskMutex<T> {
        PrimaskMutex {
            data: UnsafeCell::new(data),
        }
    }

    /// Executes the closure `critical_section` without interrupts.
    ///
    /// If interrupts were enabled before entering the critical section, the interrupts are enabled
    /// again after the critical section
    ///
    /// # Examples
    /// ```
    /// let x = 5;
    /// let mutex = PrimaskMutex::new(x);
    /// mutex.lock(|data| {
    ///     // Interrupt free section
    ///     // Safe, because 'atomic' block
    ///     data += 5;
    /// });
    /// // Interrupts are enabled again, if interrupts was enabled before the critical section
    /// ```
    pub fn lock<F, R>(&self, critical_section: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        // PRIMASK = 1 => Prevents the activation of all exceptions with configurable priority
        let primask_active = primask::read().is_active();
        interrupt::disable();

        let result = critical_section(unsafe { &mut *self.data.get() });

        // If PRIMASK was active (Interrupts enabled) then enable interrupts again
        if primask_active {
            unsafe { interrupt::enable() };
        }

        result
    }
}
