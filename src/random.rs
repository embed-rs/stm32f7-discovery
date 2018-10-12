//! # Driver for the stm32f7_discovery rng module
//! Use at your own risk. AND NOT FOR CRYPTOGRAPHIC PURPOSES !!!einself
//!
//! Example
//!
//! ```
//! let mut random_gen = rng::init(rng, rcc);
//! match random_gen.poll_and_get() {
//!
//!         Ok(random_number) => {
//!             println!("Got a random number {}", random_number);
//!         }
//!
//!         Err(_) => {
//!             println!("Something went wrong");
//!         }
//!     }
//! ```
//! Since for disabling the rng, some rcc clock on the AHB2 Bus must be disabled as well.
//! Therefore use .disable(rcc) after you are done.
//!
//! ```
//! random_gen.disable(rcc);
//! ```
//!
//! Iter is currently not implemented. Pull Requests welcome!

use core::ops::Drop;
use core::result::Result;
use stm32f7::stm32f7x6::{RCC, RNG};

/// Contains state as well as the Rng Struct from embedded::board.
pub struct Rng<'a> {
    last_number: u32,
    counter: u32,
    board_rng: &'a mut RNG,
}

/// Any of the errors (except AlreadyEnabled) can usually be resolved by initializing this
/// struct again.
#[derive(Debug)]
pub enum ErrorType {
    CECS,
    SECS,
    CEIS,
    SEIS,
    AlreadyEnabled,
    NotReady,
}

impl<'a> Rng<'a> {
    ///! This will take semi-ownership (with &'static) for the rng struct
    /// from board::rng.
    pub fn init(rng: &'a mut RNG, rcc: &mut RCC) -> Result<Self, ErrorType> {
        let control_register = rng.cr.read().rngen();
        if control_register.bit_is_set() {
            return Err(ErrorType::AlreadyEnabled);
        }

        let rng = Rng {
            last_number: 0x0,
            counter: 0x0,
            board_rng: rng,
        };
        rcc.ahb2enr.modify(|_, w| w.rngen().set_bit());

        rng.board_rng.cr.modify(|_, w| {
            w.ie().clear_bit();
            w.rngen().set_bit();
            w
        });

        Ok(rng)
    }

    /// For Testing purposes. Do not use except for debugging!
    pub fn tick(&mut self) -> u32 {
        self.poll_and_get().unwrap_or(0)
    }

    /// Actually try to acquire some random number
    /// Returns Ok(number) or Err!
    pub fn poll_and_get(&mut self) -> Result<u32, ErrorType> {
        let status = self.board_rng.sr.read();

        if status.ceis().bit_is_set() {
            self.reset();
            return Err(ErrorType::CEIS);
        }
        if status.seis().bit_is_set() {
            self.reset();
            return Err(ErrorType::SEIS);
        }

        if status.cecs().bit_is_set() {
            return Err(ErrorType::CECS);
        }
        if status.secs().bit_is_set() {
            self.reset();
            return Err(ErrorType::SECS);
        }
        if status.drdy().bit_is_set() {
            let data = self.board_rng.dr.read().rndata().bits();
            if data != self.last_number {
                self.last_number = data;
                self.counter = 0;
                return Ok(data);
            }
        }
        self.counter += 1;
        if self.counter > 80 {
            self.reset();
            self.counter = 0;
        }
        // data was not ready, try again!
        Err(ErrorType::NotReady)
    }

    pub fn reset(&mut self) {
        self.board_rng.cr.modify(|_, w| w.rngen().clear_bit());
        self.board_rng.cr.modify(|_, w| w.ie().clear_bit());
        self.board_rng.cr.modify(|_, w| w.rngen().set_bit());
    }

    fn disable_cr(&mut self, rcc: &mut RCC) {
        self.board_rng.cr.modify(|_, w| w.rngen().clear_bit());
        self.board_rng.cr.modify(|_, w| w.ie().clear_bit());
        rcc.ahb2enr.modify(|_, w| w.rngen().clear_bit());
    }

    pub fn disable(mut self, rcc: &mut RCC) {
        use core::mem;
        self.disable_cr(rcc);
        mem::forget(self);
    }
}

impl<'a> Drop for Rng<'a> {
    /// PANICS EVERYTIME! Use .disable(rcc) explicitly!
    fn drop(&mut self) {
        panic!("Use .disable() method on your random struct!");
    }
}
