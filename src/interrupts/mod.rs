//! This module implements the `InterruptController` trait of
//! the `interrupture` crate in order to offer the high level
//! interrupt interface on `stm32f7x6`

pub use interrupture_stm32f7x6::*;

pub mod primask_mutex;

/// Wait for interrupt.
///
/// This function calls the `wfi` assembler command of the cortex-m processors.
pub unsafe fn wfi() {
    ::cortex_m::asm::wfi();
}
