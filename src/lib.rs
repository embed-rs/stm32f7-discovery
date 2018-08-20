#![no_std]
#![feature(try_from)]
#![feature(const_fn)]
#![feature(trusted_len)]
#![feature(alloc)]

#[macro_use]
extern crate alloc;
extern crate arrayvec;
extern crate cortex_m;
extern crate font8x8;
extern crate spin;
extern crate stm32f7;
#[macro_use]
extern crate bitflags;
extern crate bit_field;
extern crate byteorder;
extern crate smoltcp;
extern crate volatile;

pub mod ethernet;
pub mod gpio;
pub mod i2c;
pub mod init;
pub mod lcd;
pub mod random;
pub mod sd;
pub mod system_clock;
pub mod touch;
