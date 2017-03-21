#![feature(lang_items)]
#![feature(const_fn)]
#![feature(trusted_len)]

#![no_std]

// memcpy, memmove, etc.
extern crate rlibc;
// various compiler builtins such as `__aeabi_memcpy4`
extern crate compiler_builtins_snapshot;
// hardware register structs with accessor methods
pub extern crate embedded_stm32f7 as board;
pub extern crate embedded;
// low level access to the cortex-m cpu
extern crate cortex_m;
// volatile wrapper types
extern crate volatile;
extern crate arrayvec;

pub mod exceptions;
pub mod interrupts;
pub mod system_clock;
pub mod sdram;
pub mod lcd;
pub mod i2c;
pub mod audio;
pub mod touch;
