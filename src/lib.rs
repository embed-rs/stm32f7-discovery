#![no_std]
#![feature(try_from)]
#![feature(trusted_len)]
#![feature(alloc)]
#![feature(optin_builtin_traits)]
#![feature(futures_api)]
#![feature(generator_trait)]
#![feature(pin)]
#![feature(arbitrary_self_types)]
#![feature(drain_filter)]
#![feature(never_type)]
#![feature(generators)]
#![feature(async_await)]

#[macro_use]
extern crate alloc;
extern crate arrayvec;
extern crate cortex_m;
extern crate font8x8;
extern crate spin;
extern crate stm32f7;
#[macro_use]
extern crate bitflags;
extern crate bare_metal;
extern crate bit_field;
extern crate byteorder;
extern crate cortex_m_rt as rt;
extern crate smoltcp;
extern crate volatile;
extern crate embedded_hal;
extern crate futures;

#[macro_use]
pub mod future_runtime;
#[macro_use]
pub mod lcd;
pub mod ethernet;
pub mod gpio;
pub mod i2c;
pub mod init;
pub mod interrupts;
pub mod random;
pub mod sd;
pub mod system_clock;
pub mod touch;
pub mod task_runtime;
pub mod mpsc_queue;
pub mod future_mutex;
