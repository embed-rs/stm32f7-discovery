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

use crate::rt::exception;
use alloc::boxed::Box;
use bare_metal::Nr;
use core::convert::{TryFrom, TryInto};
use core::intrinsics::transmute;
use core::marker::PhantomData;
use core::{fmt, ptr};
pub use stm32f7::stm32f7x6::Interrupt as InterruptRequest;
use stm32f7::stm32f7x6::{NVIC, NVIC_STIR};

pub mod primask_mutex;

/// The default interrupt handler that is called for all uncaught IRQs.
#[exception]
fn DefaultHandler(irqn: i16) {
    if irqn < 0 {
        panic!("Unhandled exception (IRQn = {})", irqn);
    } else {
        unsafe {
            match ISRS[usize::try_from(irqn).unwrap()] {
                Some(ref mut isr) => isr(),
                None => default_interrupt_handler(irqn.try_into().unwrap()),
            }
        }
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
pub enum Error {
    /// The error type which is returned when an interrupt is registered that is already being used.
    InterruptAlreadyInUse(InterruptRequest),
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InterruptAlreadyInUse(interrupt_request) => {
                // fixme: use Debug implementation when available
                writeln!(
                    f,
                    "Error::InterruptAlreadyInUse({})",
                    interrupt_request.nr()
                )
            }
        }
    }
}

/// The `InterruptHandle` is used to access and configure an active interrupt.
pub struct InterruptHandle<T> {
    _data_type: PhantomData<T>,
    irq: InterruptRequest,
}

impl<T> InterruptHandle<T> {
    fn new(irq: InterruptRequest) -> Self {
        InterruptHandle {
            irq: irq,
            _data_type: PhantomData,
        }
    }
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
pub struct InterruptTable<'a> {
    _lifetime: PhantomData<&'a ()>,
    nvic: &'a mut NVIC,
    nvic_stir: &'a mut NVIC_STIR,
    data: [*mut (); 98],
}

impl<'a> !Sync for InterruptTable<'a> {}

impl<'a> Drop for InterruptTable<'a> {
    fn drop(&mut self) {
        let mut some_left = false;
        unsafe {
            DEFAULT_INTERRUPT_HANDLER = None;
            for (i, isr) in ISRS.iter().enumerate() {
                some_left = some_left || isr.is_some();
                self.disable_interrupt(InterruptId(u8::try_from(i).unwrap()));
            }
        }
        if some_left {
            panic!("Disable interrupts first");
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
pub fn scope<'a, F, C, R>(
    nvic: &'a mut NVIC,
    nvic_stir: &'a mut NVIC_STIR,
    default_handler: F,
    code: C,
) -> R
where
    F: FnMut(u8) + Send,
    C: FnOnce(&mut InterruptTable<'a>) -> R,
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
        nvic: nvic,
        nvic_stir: nvic_stir,
        data: [ptr::null_mut(); 98],
    };
    // When the *code(self)* panics, the programm ends in an endless loop with disabled interrupts
    // and never returns. So the state of the ISRS does't matter.
    code(&mut interrupt_table)

    // Drop is called
}

impl<'a> InterruptTable<'a> {
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
        irq: InterruptRequest,
        priority: Priority,
        mut isr: F,
    ) -> Result<InterruptHandle<()>, Error>
    where
        F: FnMut() + 'a + Send,
    {
        self.register_owned(irq, priority, (), move |_| isr())
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
        irq: InterruptRequest,
        priority: Priority,
        owned_data: T,
        mut isr: F,
    ) -> Result<InterruptHandle<T>, Error>
    where
        T: Send,
        F: FnMut(&mut T) + 'a + Send,
    {
        let irq_number = irq.nr();
        if unsafe { ISRS[irq_number as usize].is_some() } {
            return Err(Error::InterruptAlreadyInUse(irq));
        }
        // Insert data only, when interrupt isn't used, therefore nobody reads the data
        // => no dataraces
        self.data[irq_number as usize] = Box::into_raw(Box::new(owned_data)) as *mut ();

        // transmute::<Box<FnMut()>, Box<FnMut() + 'static + Send>> is safe, because of the
        // drop implementation of InterruptTable ('static is not needed for closure)
        // and alway only one isr can access the data (Send is not needed for closure)
        let isr = unsafe {
            let parameter = &mut *(self.data[irq_number as usize] as *mut T);
            transmute::<Box<FnMut()>, Box<FnMut() + 'static + Send>>(Box::new(move || {
                isr(parameter);
            }))
        };
        let interrupt_handle = self.insert_boxed_isr(irq, isr)?;
        self.set_priority(&interrupt_handle, priority);
        self.enable_interrupt(InterruptId(irq_number));

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
        irq: InterruptRequest,
        priority: Priority,
        isr: F,
        code: C,
    ) -> Result<(), Error>
    where
        F: FnMut() + Send,
        C: FnOnce(&mut InterruptTable),
    {
        let irq_number = irq.nr();
        if unsafe { ISRS[irq_number as usize].is_some() } {
            return Err(Error::InterruptAlreadyInUse(irq));
        }

        // Insert a `()` into data to simplify `unregister`
        self.data[irq_number as usize] = Box::into_raw(Box::new(())) as *mut ();

        // Safe: Isr is removed from the static array after the closure *code* is executed.
        // When the *code(self)* panics, the programm ends in an endless loop with disabled
        // interrupts and never returns. So the state of the ISRS does't matter.
        let isr = unsafe {
            transmute::<Box<FnMut() + Send>, Box<FnMut() + 'static + Send>>(Box::new(isr))
        };
        let interrupt_handle = self.insert_boxed_isr::<()>(irq, isr)?;
        self.set_priority(&interrupt_handle, priority);
        self.enable_interrupt(InterruptId(irq_number));

        code(self);

        self.unregister(interrupt_handle);

        Ok(())
    }

    fn insert_boxed_isr<T>(
        &mut self,
        irq: InterruptRequest,
        isr_boxed: Box<FnMut() + 'static + Send>,
    ) -> Result<InterruptHandle<T>, Error> {
        let irq_number = irq.nr();
        // Check if interrupt already in use
        if unsafe { ISRS[irq_number as usize].is_some() } {
            return Err(Error::InterruptAlreadyInUse(irq));
        }
        unsafe {
            ISRS[irq_number as usize] = Some(isr_boxed);
        }

        Ok(InterruptHandle::new(irq))
    }

    fn enable_interrupt<I: Nr>(&mut self, irq: I) {
        self.nvic.enable(irq);
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
    pub fn unregister<T>(&mut self, interrupt_handle: InterruptHandle<T>) -> T {
        let irq = InterruptId(interrupt_handle.irq.nr());
        let irq_number = irq.nr();
        self.disable_interrupt(irq);

        let data = self.data[irq_number as usize];
        self.data[irq_number as usize] = ptr::null_mut();
        *unsafe { Box::from_raw(data as *mut T) }
    }

    fn disable_interrupt<I: Nr>(&mut self, irq: I) {
        let irq_number = irq.nr();
        self.nvic.disable(irq);
        unsafe {
            ISRS[usize::from(irq_number)] = None;
        }
    }

    /// Sets the priority of the interrupt corresponding to the `interrupt_handle`.
    pub fn set_priority<T>(&mut self, interrupt_handle: &InterruptHandle<T>, priority: Priority) {
        let irq = InterruptId(interrupt_handle.irq.nr());
        // The STM32F7 only supports 16 priority levels
        // Assert that priority < 16
        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used
        // for priority.
        let priority = (priority as u8) << 4;

        unsafe { self.nvic.set_priority(irq, priority) };
    }

    /// Returns the priority of the interrupt corresponding to the `interrupt_handle`.
    pub fn get_priority<T>(&self, interrupt_handle: &InterruptHandle<T>) -> Priority {
        let irq = InterruptId(interrupt_handle.irq.nr());

        let res = NVIC::get_priority(irq);

        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used
        // for priority.
        match Priority::from_u8(res >> 4) {
            Ok(priority) => priority,
            Err(PriorityDoesNotExistError(prio_number)) => {
                unreachable!("Priority {} does not exist", prio_number)
            }
        }
    }

    /// Clears the pending state of the interrupt corresponding to the `interrupt_handle`.
    pub fn clear_pending_state<T>(&mut self, interrupt_handle: &InterruptHandle<T>) {
        let irq = InterruptId(interrupt_handle.irq.nr());
        NVIC::unpend(irq);
    }

    /// Sets the pending state of the interrupt corresponding to the `interrupt_handle`.
    pub fn set_pending_state<T>(&mut self, interrupt_handle: &InterruptHandle<T>) {
        let irq = InterruptId(interrupt_handle.irq.nr());
        NVIC::pend(irq);
    }

    /// Returns the pending state of the interrupt corresponding to the `interrupt_handle`.
    pub fn get_pending_state<T>(&self, interrupt_handle: &InterruptHandle<T>) -> bool {
        let irq = InterruptId(interrupt_handle.irq.nr());
        NVIC::is_pending(irq)
    }

    /// Triggers the given interrupt `irq`.
    pub fn trigger(&mut self, irq: InterruptRequest) {
        self.nvic_stir
            .stir
            .write(|w| unsafe { w.intid().bits(irq.nr().into()) });
    }
}

/// Possible interrupt priorities of the stm32f7.
///
/// Lower number means higher priority:
/// `P1` has a higher priority than e.g. `P2`, `P5`, ...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    /// Priority 0
    P0 = 0,
    /// Priority 1
    P1,
    /// Priority 2
    P2,
    /// Priority 3
    P3,
    /// Priority 4
    P4,
    /// Priority 5
    P5,
    /// Priority 6
    P6,
    /// Priority 7
    P7,
    /// Priority 8
    P8,
    /// Priority 9
    P9,
    /// Priority 10
    P10,
    /// Priority 11
    P11,
    /// Priority 12
    P12,
    /// Priority 13
    P13,
    /// Priority 14
    P14,
    /// Priority 15
    P15,
}
struct PriorityDoesNotExistError(u8);

impl Priority {
    /// Converts a u8 to a Priority.
    ///
    /// Returns an `Err` when no variant with the given `priority` exists.
    // use FromPrimitive?
    fn from_u8(priority: u8) -> Result<Priority, PriorityDoesNotExistError> {
        use self::Priority::*;
        match priority {
            0 => Ok(P0),
            1 => Ok(P1),
            2 => Ok(P2),
            3 => Ok(P3),
            4 => Ok(P4),
            5 => Ok(P5),
            6 => Ok(P6),
            7 => Ok(P7),
            8 => Ok(P8),
            9 => Ok(P9),
            10 => Ok(P10),
            11 => Ok(P11),
            12 => Ok(P12),
            13 => Ok(P13),
            14 => Ok(P14),
            15 => Ok(P15),
            _ => Err(PriorityDoesNotExistError(priority)),
        }
    }
}

/// Wait for interrupt.
///
/// This function calls the `wfi` assembler command of the cortex-m processors.
pub unsafe fn wfi() {
    ::cortex_m::asm::wfi();
}

struct InterruptId(u8);

unsafe impl Nr for InterruptId {
    fn nr(&self) -> u8 {
        self.0
    }
}
