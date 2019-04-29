//! This module implements the `InterruptController` trait of
//! the `interrupture` crate in order to offer the high level
//! interrupt interface on `stm32f7x6`

use interrupture::handle_isr;
use core::convert::TryFrom;
use crate::rt::exception;

pub mod primask_mutex;

pub use stm32f7::stm32f7x6::Interrupt as InterruptRequest;
use stm32f7::stm32f7x6::{NVIC, NVIC_STIR};
use bare_metal::Nr;

/// A convenience wrapper around `interrupture::scope` for `stm32f7x6`
pub fn scope<'a, F, C, R>(
    nvic: &'a mut NVIC,
    nvic_stir: &'a mut NVIC_STIR,
    default_handler: F,
    code: C,
) -> R
where
    F: FnMut(u8) + Send,
    C: FnOnce(&mut interrupture::InterruptTable<'a, Ic<'a>>) -> R,
{
    let ic = Ic { nvic, nvic_stir };
    interrupture::scope(ic, default_handler, code)
}

#[doc(hidden)]
/// This type only exists for the `InterruptController` trait bound on closure
/// of `scope`, do not use directly, you will never interact with it directly anyway.
pub struct Ic<'a> {
    nvic: &'a mut NVIC,
    nvic_stir: &'a mut NVIC_STIR,
}

// HACK: Nr should be more convenient to use (e.g. have some forwarding impls)
struct NrWrap<'a, T: Nr>(&'a T);
unsafe impl<'a, T: Nr> Nr for NrWrap<'a, T> {
    fn nr(&self) -> u8 {
        self.0.nr()
    }
}

impl<'a> interrupture::InterruptController for Ic<'a> {
    type Request = InterruptRequest;
    type Priority = Priority;
    fn trigger(&mut self, irq: &Self::Request) {
        self.nvic_stir
            .stir
            .write(|w| unsafe { w.intid().bits(irq.nr().into()) });
    }
    fn is_pending(irq: &Self::Request) -> bool {
        NVIC::is_pending(NrWrap(irq))
    }
    fn pend(irq: &Self::Request) {
        NVIC::pend(NrWrap(irq));
    }
    fn unpend(irq: &Self::Request) {
        NVIC::unpend(NrWrap(irq));
    }
    fn get_priority(irq: &Self::Request) -> Self::Priority {
        let res = NVIC::get_priority(NrWrap(irq));

        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used
        // for priority.
        match Priority::from_u8(res >> 4) {
            Ok(priority) => priority,
            Err(PriorityDoesNotExistError(prio_number)) => {
                unreachable!("Priority {} does not exist", prio_number)
            }
        }
    }
    fn set_priority(&mut self, irq: &Self::Request, priority: Self::Priority) {
        // The STM32F7 only supports 16 priority levels
        // Assert that priority < 16
        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used
        // for priority.
        let priority = (priority as u8) << 4;

        unsafe { self.nvic.set_priority(NrWrap(irq), priority) };
    }
    fn disable(&mut self, irq: &Self::Request) {
        self.nvic.disable(NrWrap(irq));
    }
    fn enable(&mut self, irq: &Self::Request) {
        self.nvic.enable(NrWrap(irq));
    }
}

/// The default interrupt handler that is called for all uncaught IRQs.
#[exception]
fn DefaultHandler(irqn: i16) {
    if let Ok(irqn) = u8::try_from(irqn) {
        handle_isr(irqn)
    } else {
        panic!("Unhandled exception (IRQn = {})", irqn);
    }
}

/// Wait for interrupt.
///
/// This function calls the `wfi` assembler command of the cortex-m processors.
pub unsafe fn wfi() {
    ::cortex_m::asm::wfi();
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

