use core::cell::UnsafeCell;

pub struct PrimaskMutex<T> {
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for PrimaskMutex<T> {}
unsafe impl<T: Send> Sync for PrimaskMutex<T> {}

impl<T> PrimaskMutex<T> {
    pub fn new(data: T) -> PrimaskMutex<T> {
        PrimaskMutex { data: UnsafeCell::new(data) }
    }

    pub fn lock<F>(&self, critical_section: F) 
        where F: FnOnce(&mut T)
    {
        // PRIMASK = 1 => Prevents the activation of all exceptions with configurable priority
        let primask = unsafe { ::cortex_m::register::primask::read() } & 1 == 1;
        unsafe { ::cortex_m::interrupt::disable() };

        critical_section(unsafe { &mut *self.data.get() });

        // If PRIMASK was '0' (Interrupts enabled) then enable interrupts again
        if !primask {
            unsafe { ::cortex_m::interrupt::enable() };
        }
    }
}
