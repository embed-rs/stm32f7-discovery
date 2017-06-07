use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut, Drop};

pub struct PrimaskMutex<T> {
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for PrimaskMutex<T> {}
unsafe impl<T: Send> Sync for PrimaskMutex<T> {}

pub struct PrimaskMutexGuard<'a, T: 'a> {
    _data: &'a mut T,
    prev: bool,
}

impl<'a, T> PrimaskMutex<T> {
    pub fn new(data: T) -> PrimaskMutex<T> {
        PrimaskMutex {
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&'a self) -> PrimaskMutexGuard<'a, T> {
        let primask =  if unsafe { ::cortex_m::register::primask::read() } & 1 == 1 {
            true
        } else {
            false
        };
        unsafe { ::cortex_m::interrupt::disable() };

        PrimaskMutexGuard {
            _data: unsafe {&mut *self.data.get()},
            prev: primask,
        }
    }
}

impl<'a, T: 'a> Drop for PrimaskMutexGuard<'a, T> {
    fn drop(&mut self) {
        if !self.prev {
            unsafe { ::cortex_m::interrupt::enable() };
        }
    }
}

impl<'a, T: 'a> Deref for PrimaskMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self._data
    }
}

impl<'a, T: 'a> DerefMut for PrimaskMutexGuard<'a, T> {
    
    fn deref_mut(&mut self) -> &mut T {
        self._data
    }
}