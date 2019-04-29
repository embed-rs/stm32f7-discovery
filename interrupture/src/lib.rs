//! Safe and "free of data races" interrupt system.
//!
//! The interrupt system features are:
//!
//! - **Ownership based interrupt management**. The `InterruptTable` owns the nvic register
//! and thus it is the only one that can access and change the interrupt controller.
//!
//! - **Easy to use closure-based ISR registration**. Closures can be registered as interrupt
//! service routine.
//!
//! - **Free of data races**. Thanks to Rust `Send` and `Sync` concept, the interrupt system
//! is free of data races. Shared mutable access on a variable must be synchronized with a
//! PrimaskMutex, otherwise the compilation fails.
//!
//! - **Scoped IRSs with access to the enviroment**. It is guaranteed that the closure is
//! unregistered at the end of the scope. Thus it is safe to access the parent stack in the
//! interrupt service routine.

#![no_std]
#![warn(missing_docs)]
#![deny(clippy::all)]
#![feature(alloc_prelude)]
#![feature(optin_builtin_traits)]

extern crate alloc;

use alloc::boxed::Box;
use core::intrinsics::transmute;
use core::marker::PhantomData;
use core::ptr;
use core::mem;
use bare_metal::Nr;

#[inline(always)]
/// Call this function from your `#[exception]` default handler in order to thread the
/// interrupts through to this crate's handler code.
pub fn handle_isr(irqn: u8) {
    match unsafe { &mut ISRS[irqn as usize] } {
        Some(isr) => isr(),
        None => default_interrupt_handler(irqn)
    }
}

static mut ISRS: [Option<Box<FnMut()>>; 98] = [
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None,
];

/// Default interrupt handler
static mut DEFAULT_INTERRUPT_HANDLER: Option<Box<FnMut(u8)>> = None;

// Unreachable at the moment (only when interrupt was enabled before InterruptTable got
// ownership...)
fn default_interrupt_handler(irq: u8) {
    unsafe {
        match DEFAULT_INTERRUPT_HANDLER {
            Some(ref mut handler) => handler(irq),
            None => panic!("No default handler"),
        }
    }
}

/// The error type that can occur when handling with interrupts.
#[derive(Debug)]
pub enum Error {
    /// The error type which is returned when an interrupt is registered that is already being used.
    ///
    /// Gives you back the index that you used to try to register the interrupt
    InterruptAlreadyInUse(u8),
}

/// The `InterruptHandle` is used to access and configure an active interrupt.
pub struct InterruptHandle<T, REQ> {
    _data_type: PhantomData<T>,
    irq: REQ,
}

impl<T, REQ> InterruptHandle<T, REQ> {
    const fn new(irq: REQ) -> Self {
        InterruptHandle {
            irq,
            _data_type: PhantomData,
        }
    }
}

/// A low level API for interrupts. Implement this API
/// for your hardware in order to get access to the more
/// convenient `crossbeam`-like API.
pub trait InterruptController {
    /// An interrupt identifier. Should never contain values that can't be represented as a `u8`.
    type Request: Nr;
    /// A priority identifier. Opaquely used by `interrupture` and just forwarded back to you.
    type Priority;

    /// Causes an interrupt routine to be invoked by making the hardware believe the
    /// interrupt was triggered.
    ///
    /// Essentially this is a software interrupt trigger.
    fn trigger(&mut self, irq: &Self::Request);

    /// Returns the pending state of the interrupt
    fn is_pending(irq: &Self::Request) -> bool;

    /// Sets the pending state of the interrupt to `true`
    fn pend(irq: &Self::Request);

    /// Sets the pending state of the interrupt to `false`
    fn unpend(irq: &Self::Request);

    /// Fetches the priority of the given interrupt
    fn get_priority(irq: &Self::Request) -> Self::Priority;

    /// Sets a the new priority of the given interrupt
    fn set_priority(&mut self, irq: &Self::Request, priority: Self::Priority);

    /// Disables the given interrupt
    fn disable(&mut self, irq: &Self::Request);

    /// Enables the given interrupt
    fn enable(&mut self, irq: &Self::Request);
}

/// The `InterruptTable` guarantees safe and 'free of data races' use of interrupts.
///
/// To ensure that no data races can occur, it uses the Send and Sync concurrency concept from Rust.
/// The `InterruptTable` can only be used in the `code(&mut InterruptTable)` function passed to the
/// `scope` function, to ensure that `InterruptTable.drop()` is called.
///
/// # Examples
/// ```
/// interrupts::scope(nvic, |irq| { hprintln!("Default handler: {}", irq) },
///     use stm32f7::interrupts::interrupt_request::InterruptRequest::Tim7;
///     use interrupts::Priority::P1;
///     |interrupt_table| {
///
///         let _ = interrupt_table.register(Tim7, P1, || {
///         hprintln!("Interrupt handler for Tim7");
///     });
/// });
/// ```
pub struct InterruptTable<'a, IC: InterruptController> {
    _lifetime: PhantomData<&'a ()>,
    ic: IC,
    data: [*mut (); 98],
}

impl<'a, IC> !Sync for InterruptTable<'a, IC> {}

impl<'a, IC: InterruptController> Drop for InterruptTable<'a, IC> {
    fn drop(&mut self) {
        unsafe {
            DEFAULT_INTERRUPT_HANDLER = None;
            for (i, isr) in ISRS.iter().enumerate() {
                assert!(
                    isr.is_none(),
                    "Interrupt {} is still enabled while the InterruptTable is being dropped",
                    i,
                );
            }
        }
    }
}

/// Creates a new scope, to guarantee that the `InterruptTable` constructor is called.
///
/// # Examples
/// ```rust
/// fn main(hw: board::Hardware) -> ! {
///     // Extract hardware
///     let board::Hardware {
///         rcc,
///         pwr,
///         flash,
///         nvic,
///         ..
///     } = hw;
///
///     // Configure system clock
///     system_clock::init(rcc, pwr, flash);
///
///     use stm32f7::interrupts::interrupt_request::InterruptRequest::Tim7;
///     use interrupts::Priority::P1;
///     // Open scope with interrupt support
///     interrupts::scope(nvic, |irq| { hprintln!("Default handler: {}", irq) },
///         |interrupt_table| {
///             let _ = interrupt_table.register(Tim7, P1, || {
///                 hprintln!("Interrupt handler for Tim7");
///         });
///     });
///    loop{}
/// }
/// ```
///
/// # Panics
/// Panics if an interrupt is enabled and is not disabled after use in `code()`
pub fn scope<'a, IC, F, C, R>(
    ic: IC,
    default_handler: F,
    code: C,
) -> R
where
    IC: InterruptController,
    F: FnMut(u8) + Send,
    C: FnOnce(&mut InterruptTable<'a, IC>) -> R,
{
    unsafe {
        debug_assert!(DEFAULT_INTERRUPT_HANDLER.is_none());
        DEFAULT_INTERRUPT_HANDLER = Some(transmute::<
            Box<FnMut(u8) + Send>,
            Box<FnMut(u8) + 'static>,
        >(Box::new(default_handler)));
    }

    let mut interrupt_table = InterruptTable {
        _lifetime: PhantomData,
        ic,
        data: [ptr::null_mut(); 98],
    };
    // When the *code(self)* panics, the programm ends in an endless loop with disabled interrupts
    // and never returns. So the state of the ISRS does't matter.
    code(&mut interrupt_table)

    // Drop is called
}

impl<'a, IC: InterruptController> InterruptTable<'a, IC> {
    /// Registers an interrupt with the lifetime of the `InterruptTable`.
    ///
    /// # Examples
    /// ```
    /// use stm32f7::interrupts::interrupt_request::InterruptRequest::Tim7;
    /// use interrupts::Priority::P1;
    /// interrupts::scope(nvic, |irq| { hprintln!("Default handler: {}", irq) },
    ///     |interrupt_table| {
    ///             let interrupt_handle = interrupt_table.register(Tim7, P1,
    ///             || {
    ///                 // Isr for interrupt `Tim7`
    ///             }).expect("Interrupt already used");
    ///
    ///             /* Code that needs interrupt `Tim7` */
    ///
    ///             // Unregister interrupt and get back the ownership to `data`
    ///             let data = interrupt_table.unregister(interrupt_handle);
    ///             assert!(data.is_none());
    /// });
    /// ```
    pub fn register<F>(
        &mut self,
        irq: IC::Request,
        priority: IC::Priority,
        mut isr: F,
    ) -> Result<InterruptHandle<(), IC::Request>, Error>
    where
        F: FnMut() + 'a + Send,
    {
        self.register_owned(irq, priority, (), move |_| isr())
    }

    fn err_if_irq_in_use(&self, irq: u8) -> Result<(), Error> {
        if unsafe { ISRS[usize::from(irq)].is_some() } {
            Err(Error::InterruptAlreadyInUse(irq))
        } else {
            Ok(())
        }
    }

    /// Registers an interrupt with the lifetime of the `InterruptTable` and pass ownership
    /// of a variable `owned_data: T` that is passed to the `isr(&mut T)` when the corresponding
    /// interupt `irq` occur.
    ///
    /// The ownership of the data `owned_data` is returned in the `unregister()` function
    ///
    /// # Examples
    /// ```
    /// use stm32f7::interrupts::interrupt_request::InterruptRequest::Tim7;
    /// use interrupts::Priority::P1;
    /// interrupts::scope(nvic, |irq| { hprintln!("Default handler: {}", irq) },
    ///     |interrupt_table| {
    ///             let data: SomeData = ...;
    ///             let interrupt_handle = interrupt_table.register_owned(Tim7, P1, data,
    ///             |owned_data| {
    ///                 // Isr for interrupt `Tim7`
    ///                 owned_data.do_anything();
    ///             }).expect("Interrupt already used");
    ///
    ///             /* Code that needs interrupt `Tim7` */
    ///
    ///             // Unregister interrupt and get back the ownership to `data`
    ///             let data = interrupt_table.unregister(interrupt_handle).unwrap();
    /// });
    /// ```
    pub fn register_owned<F, T>(
        &mut self,
        irq: IC::Request,
        priority: IC::Priority,
        owned_data: T,
        mut isr: F,
    ) -> Result<InterruptHandle<T, IC::Request>, Error>
    where
        T: Send,
        F: FnMut(&mut T) + 'a + Send,
    {
        self.err_if_irq_in_use(irq.nr())?;
        // Insert data only, when interrupt isn't used, therefore nobody reads the data
        // => no dataraces
        self.data[usize::from(irq.nr())] = Box::into_raw(Box::new(owned_data)) as *mut ();

        // transmute::<Box<FnMut()>, Box<FnMut() + 'static + Send>> is safe, because of the
        // drop implementation of InterruptTable ('static is not needed for closure)
        // and alway only one isr can access the data (Send is not needed for closure)
        let isr = unsafe {
            let parameter = &mut *(self.data[usize::from(irq.nr())] as *mut T);
            transmute::<Box<FnMut()>, Box<FnMut() + 'static + Send>>(Box::new(move || {
                isr(parameter);
            }))
        };
        let interrupt_handle = self.insert_boxed_isr(irq, isr)?;
        self.set_priority(&interrupt_handle, priority);
        self.ic.enable(&interrupt_handle.irq);

        Ok(interrupt_handle)
    }

    /// Registers a temporary interrupt that is enabled while the function `code` is running.
    ///
    /// `isr()` is called, when interrupt `irq` occur.
    /// Interrupt `irq` is disabled again after this function.
    ///
    /// # Examples
    /// ```
    /// use stm32f7::interrupts::interrupt_request::InterruptRequest::Tim7;
    /// use interrupts::Priority::P1;
    /// // Open scope with interrupt support
    /// interrupts::scope(nvic, |irq| { hprintln!("Default handler: {}", irq) },
    ///     |interrupt_table| {
    ///         let a = &mut some_data;
    ///         interrupt_table.with_interrupt(Tim7, P1,
    ///             || { // Isr for interrupt `Tim7``
    ///                 some_data.do_anything();
    ///             },
    ///             || { /* code that needs that interrupt `Tim7` to be set */ }
    ///         );
    ///         // interrupt is not set anymore, `some_data` is available again
    /// });
    /// ```
    pub fn with_interrupt<F, C>(
        &mut self,
        irq: IC::Request,
        priority: IC::Priority,
        isr: F,
        code: C,
    ) -> Result<(), Error>
    where
        F: FnMut() + Send,
        C: FnOnce(&mut InterruptTable<IC>),
    {
        self.err_if_irq_in_use(irq.nr())?;

        // Insert a `()` into data to simplify `unregister`
        self.data[usize::from(irq.nr())] = Box::into_raw(Box::new(())) as *mut ();

        // Safe: Isr is removed from the static array after the closure *code* is executed.
        // When the *code(self)* panics, the programm ends in an endless loop with disabled
        // interrupts and never returns. So the state of the ISRS does't matter.
        let isr = unsafe {
            transmute::<Box<FnMut() + Send>, Box<FnMut() + 'static + Send>>(Box::new(isr))
        };
        let interrupt_handle = self.insert_boxed_isr::<()>(irq, isr)?;
        self.set_priority(&interrupt_handle, priority);
        self.ic.enable(&interrupt_handle.irq);

        code(self);

        self.unregister(interrupt_handle);

        Ok(())
    }

    fn insert_boxed_isr<T>(
        &mut self,
        irq: IC::Request,
        isr_boxed: Box<FnMut() + 'static + Send>,
    ) -> Result<InterruptHandle<T, IC::Request>, Error> {
        self.err_if_irq_in_use(irq.nr())?;
        unsafe {
            ISRS[usize::from(irq.nr())] = Some(isr_boxed);
        }

        Ok(InterruptHandle::new(irq))
    }

    /// Unregisters the interrupt corresponding to the `interrupt_handle`.
    ///
    /// The interrupt is diabled and the binded isr is removed.
    ///
    /// Returns the ownership of the data that was passed to the `InterruptTable` with
    /// `register_owned(..., owned_data: T, ...)` or `None` when `register(...)` was used.
    ///
    /// # Examples
    /// With owned data:
    ///
    /// ```
    /// use stm32f7::interrupts::interrupt_request::InterruptRequest::Tim7;
    /// use interrupts::Priority::P1;
    /// interrupts::scope(nvic, |irq| { hprintln!("Default handler: {}", irq) },
    ///     |interrupt_table| {
    ///             let data: SomeData = ...;
    ///             let interrupt_handle = interrupt_table.register_owned(Tim7, P1, data,
    ///             |owned_data| {
    ///                 // Isr for interrupt `Tim7`
    ///                 owned_data.do_anything();
    ///             }).expect("Interrupt already used");
    ///
    ///             /* Code that needs interrupt `Tim7` */
    ///
    ///             // Unregister interrupt and get back the ownership to `data`
    ///             let data = interrupt_table.unregister(interrupt_handle);
    ///             assert!(data.is_some());
    ///             let data = data.unwrap();
    /// });
    /// ```
    /// Without owned data:
    ///
    /// ```
    /// use stm32f7::interrupts::interrupt_request::InterruptRequest::Tim7;
    /// use interrupts::Priority::P1;
    /// interrupts::scope(nvic, |irq| { hprintln!("Default handler: {}", irq) },
    ///     |interrupt_table| {
    ///             let interrupt_handle = interrupt_table.register(Tim7, P1,
    ///             || {
    ///                 // Isr for interrupt `Tim7`
    ///             }).expect("Interrupt already used");
    ///
    ///             /* Code that needs interrupt `Tim7` */
    ///
    ///             // Unregister interrupt and get back the ownership to `data`
    ///             let data = interrupt_table.unregister(interrupt_handle);
    ///             assert!(data.is_none());
    /// });
    /// ```
    pub fn unregister<T>(&mut self, interrupt_handle: InterruptHandle<T, IC::Request>) -> T {
        self.ic.disable(&interrupt_handle.irq);
        unsafe {
            ISRS[usize::from(interrupt_handle.irq.nr())] = None;
        }
        let data = mem::replace(&mut self.data[usize::from(interrupt_handle.irq.nr())], ptr::null_mut());
        *unsafe { Box::from_raw(data as *mut T) }
    }

    /// Sets the priority of the interrupt corresponding to the `interrupt_handle`.
    pub fn set_priority<T>(&mut self, interrupt_handle: &InterruptHandle<T, IC::Request>, priority: IC::Priority) {
        self.ic.set_priority(&interrupt_handle.irq, priority)
    }

    /// Returns the priority of the interrupt corresponding to the `interrupt_handle`.
    pub fn get_priority<T>(&self, interrupt_handle: &InterruptHandle<T, IC::Request>) -> IC::Priority {
        IC::get_priority(&interrupt_handle.irq)
    }

    /// Clears the pending state of the interrupt corresponding to the `interrupt_handle`.
    pub fn clear_pending_state<T>(&mut self, interrupt_handle: &InterruptHandle<T, IC::Request>) {
        IC::unpend(&interrupt_handle.irq);
    }

    /// Sets the pending state of the interrupt corresponding to the `interrupt_handle`.
    pub fn set_pending_state<T>(&mut self, interrupt_handle: &InterruptHandle<T, IC::Request>) {
        IC::pend(&interrupt_handle.irq);
    }

    /// Returns the pending state of the interrupt corresponding to the `interrupt_handle`.
    pub fn get_pending_state<T>(&self, interrupt_handle: &InterruptHandle<T, IC::Request>) -> bool {
        IC::is_pending(&interrupt_handle.irq)
    }

    /// Triggers the given interrupt `irq`.
    pub fn trigger(&mut self, irq: IC::Request) {
        self.ic.trigger(&irq)
    }
}
