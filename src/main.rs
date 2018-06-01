#![feature(alloc)]
#![feature(global_allocator)]
#![feature(lang_items)]
#![no_main]
#![no_std]

#[macro_use]
extern crate alloc;
extern crate alloc_cortex_m;
extern crate cortex_m_semihosting as sh;
#[macro_use]
extern crate stm32f746_hal as hal;

use alloc_cortex_m::CortexMHeap;
use core::fmt::{self, Write};
use hal::cortex_m::{asm, interrupt, peripheral::syst::SystClkSource, Peripherals};
use hal::rt::{self, ExceptionFrame};
use sh::hio::{self, HStdout};

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const HEAP_SIZE: usize = 1024; // in bytes

entry!(main);

fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();
    writeln!(stdout, "Hello, world!").unwrap();

    // Initialize the allocator BEFORE you use it
    unsafe { ALLOCATOR.init(rt::heap_start() as usize, HEAP_SIZE) }

    let xs = vec![1, 2, 3];

    let p = Peripherals::take().unwrap();
    let mut systick = p.SYST;

    // configures the system timer to trigger a SysTick exception every second
    systick.set_clock_source(SystClkSource::Core);
    systick.set_reload(8_000_000); // period = 1s
    systick.enable_counter();
    systick.enable_interrupt();

    loop {}
}

exception!(SysTick, sys_tick, state: Option<HStdout> = None);

fn sys_tick(state: &mut Option<HStdout>) {
    if state.is_none() {
        *state = Some(hio::hstdout().unwrap());
    }

    if let Some(hstdout) = state.as_mut() {
        hstdout.write_str(".").unwrap();
    }
}

exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

// define what happens in an Out Of Memory (OOM) condition
#[lang = "oom"]
#[no_mangle]
pub fn rust_oom() -> ! {
    if let Ok(mut hstdout) = hio::hstdout() {
        let _ = hstdout.write_str("out of memory");
    }

    // OK to fire a breakpoint here because we know the microcontroller is connected to a debugger
    asm::bkpt();

    loop {}
}

#[lang = "panic_fmt"]
#[no_mangle]
pub unsafe extern "C" fn panic_fmt(
    args: core::fmt::Arguments,
    file: &'static str,
    line: u32,
    col: u32,
) -> ! {
    interrupt::disable();

    if let Ok(mut hstdout) = hio::hstdout() {
        (|| -> Result<(), fmt::Error> {
            hstdout.write_str("panicked at '")?;
            hstdout.write_fmt(args)?;
            hstdout.write_str("', ")?;
            hstdout.write_str(file)?;
            writeln!(hstdout, ":{}:{}", line, col)
        })().ok();
    }

    // OK to fire a breakpoint here because we know the microcontroller is connected to a debugger
    asm::bkpt();

    loop {}
}
