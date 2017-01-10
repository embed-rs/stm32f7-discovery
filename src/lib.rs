#![feature(lang_items)]
#![feature(const_fn)]
#![feature(trusted_len)]
#![feature(asm)]
#![feature(alloc, collections)]

#![no_std]

// memcpy, memmove, etc.
extern crate rlibc;
// various compiler builtins such as `__aeabi_memcpy4`
extern crate compiler_builtins_snapshot;
// hardware register structs with accessor methods
extern crate embedded_stm32f7 as board;
extern crate embedded;
// low level access to the cortex-m cpu
extern crate cortex_m;
// volatile wrapper types
extern crate volatile;
// allocator
extern crate alloc_cortex_m;
extern crate alloc;
extern crate collections;
extern crate arrayvec;
extern crate bit_field;

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
#[macro_use]
pub mod semi_hosting;

#[cfg(not(test))]
#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    println_err!("\nPANIC in {} at line {}:", file, line);
    println_err!("    {}", fmt);
    loop {}
}
