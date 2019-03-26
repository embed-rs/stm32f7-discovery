//! Device library for the STM32F746NG discovery board.
//!
//! Most of the device specific code is based on the stm32f746ng [reference manual],
//! the [STM32CubeF7] package, and the [other stm32f746ng resources].
//!
//! [reference manual]: https://www.st.com/resource/en/reference_manual/dm00124865.pdf
//! [STM32CubeF7]: https://www.st.com/content/st_com/en/products/embedded-software/mcus-embedded-software/stm32-embedded-software/stm32cube-mcu-packages/stm32cubef7.html#getsoftware-scroll
//! [other stm32f746ng resources]: https://www.st.com/content/st_com/en/products/microcontrollers/stm32-32-bit-arm-cortex-mcus/stm32-high-performance-mcus/stm32f7-series/stm32f7x6/stm32f746ng.html#design-scroll

#![no_std]
#![feature(trusted_len)]
#![feature(alloc)]
#![feature(optin_builtin_traits)]
#![feature(futures_api)]
#![feature(generator_trait)]
#![feature(arbitrary_self_types)]
#![feature(drain_filter)]
#![feature(never_type)]
#![feature(generators)]
#![feature(async_await)]
#![feature(const_transmute)]
#![feature(alloc_prelude)]
#![warn(missing_docs)]

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
extern crate embedded_hal;
extern crate futures;
extern crate smoltcp;
extern crate volatile;

#[macro_use]
pub mod lcd;
pub mod ethernet;
pub mod future_mutex;
pub mod gpio;
pub mod i2c;
pub mod init;
pub mod interrupts;
pub mod mpsc_queue;
pub mod random;
pub mod sd;
pub mod system_clock;
pub mod task_runtime;
pub mod touch;
