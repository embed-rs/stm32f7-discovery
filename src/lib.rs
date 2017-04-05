#![feature(lang_items)]
#![feature(const_fn)]
#![feature(trusted_len)]
#![feature(asm)]
#![feature(alloc, collections)]
#![feature(try_from)]

#![no_std]

// memcpy, memmove, etc.
extern crate rlibc;
// hardware register structs with accessor methods
pub extern crate embedded_stm32f7 as board;
pub extern crate embedded;
// low level access to the cortex-m cpu
pub extern crate cortex_m;
// volatile wrapper types
extern crate volatile;
// allocator
extern crate alloc_cortex_m;
extern crate alloc;
#[macro_use]
extern crate collections;
extern crate arrayvec;
extern crate bit_field;
extern crate spin;
extern crate byteorder;
extern crate net;

#[macro_use]
pub mod semi_hosting;
pub mod exceptions;
pub mod interrupts;
pub mod system_clock;
pub mod sdram;
pub mod lcd;
pub mod i2c;
pub mod audio;
pub mod touch;
pub mod ethernet;
pub mod heap;

#[cfg(not(test))]
#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    println_err!("\nPANIC in {} at line {}:", file, line);
    println_err!("    {}", fmt);
    loop {}
}
