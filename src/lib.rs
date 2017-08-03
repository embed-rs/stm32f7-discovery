#![feature(lang_items)]
#![feature(const_fn)]
#![feature(trusted_len)]
#![feature(asm)]
#![feature(alloc)]
#![feature(try_from)]
#![feature(drop_types_in_const)]
#![feature(global_allocator)]

#![no_std]

// hardware register structs with accessor methods
pub extern crate embedded_stm32f7 as board;
pub use board::embedded;

// low level access to the cortex-m cpu
pub extern crate cortex_m;
// volatile wrapper types
extern crate volatile;
#[macro_use]
extern crate alloc;
extern crate arrayvec;
extern crate bit_field;
extern crate spin;
extern crate byteorder;
extern crate net;
extern crate rusttype;
extern crate alloc_cortex_m;

#[macro_use]
pub mod semi_hosting;
#[macro_use]
pub mod lcd;
pub mod exceptions;
pub mod interrupts;
pub mod system_clock;
pub mod sdram;
pub mod i2c;
pub mod audio;
pub mod touch;
pub mod ethernet;
pub mod heap;
pub mod random;

#[cfg(not(test))]
#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    use core::fmt::Write;

    hprintln_err!("\nPANIC in {} at line {}:", file, line);
    hprintln_err!("    {}", fmt);

    unsafe { lcd::stdout::force_unlock() }
    lcd::stdout::with_stdout(|stdout| {
        if let Some(ref mut stdout) = *stdout {
            let _ = writeln!(stdout, "\nPANIC in {} at line {}:", file, line);
            let _ = writeln!(stdout, "    {}", fmt);
        }
    });

    loop {}
}


use alloc_cortex_m::CortexMHeap;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();
