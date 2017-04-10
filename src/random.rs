//! # Driver for the stm32f7_discovery rng module
//! Use at your own risk. AND NOT FOR CRYPTOGRAPHIC PURPOSES !!!einself
//!
//! Example
//! ````
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
//!````
//! Since for disabling the rng, some rcc clock on the AHB2 Bus must be disabled as well.
//! Therefore use .disable(rcc) after you are done.
//! ````
//! random_gen.disable(rcc);
//! ````
//!
//! Iter is currently not implemented. Pull Requests welcome!


use core::result::Result;
use core::ops::Drop;
use ::board;


/// Contains state as well as the Rng Struct from embedded::board.
pub struct Rng {
    last_number: u32,
    counter: u32,
    board_rng: &'static mut board::rng::Rng
}


///Any of the errors (except AlreadyEnabled) can usually be resolved by initializing this
///struct again.
#[derive(Debug)]
pub enum ErrorType {
    CECS,
    SECS,
    CEIS,
    SEIS,
    AlreadyEnabled,
    NotReady
}


impl Rng {

    ///! This will take semi-ownership (with &'static) for the rng struct
    /// from board::rng.
    pub fn init(rng: &'static mut board::rng::Rng, rcc: &mut board::rcc::Rcc) -> Result<Rng, ErrorType> {

        let control_register = rng.cr.read().rngen();
        if control_register {
            return Err(ErrorType::AlreadyEnabled);
        }

        let mut rng = Rng { last_number: 0x0, counter: 0x0, board_rng: rng };
        rcc.ahb2enr.update(|r| r.set_rngen(true));

        rng.board_rng.cr.update(|r| {
            r.set_ie(false);
            r.set_rngen(true);
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

        if status.ceis() {
            self.reset();
            return Err(ErrorType::CEIS);
        }
        if status.seis() {
            self.reset();
            return Err(ErrorType::SEIS);
        }

        if status.cecs() {
            return Err(ErrorType::CECS);
        }
        if status.secs() {
            self.reset();
            return Err(ErrorType::SECS);
        }
        if status.drdy() {
            let data = self.board_rng.dr.read().rndata();
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
        self.board_rng.cr.update(|r| r.set_rngen(false));
        self.board_rng.cr.update(|r| r.set_ie(false));
        self.board_rng.cr.update(|r| r.set_rngen(true));
    }


    fn disable_cr(&mut self, rcc: &mut board::rcc::Rcc) {
        self.board_rng.cr.update(|r| r.set_rngen(false));
        self.board_rng.cr.update(|r| r.set_ie(false));
        rcc.ahb2enr.update(|r| r.set_rngen(false));
    }


    pub fn disable(mut self, rcc: &mut board::rcc::Rcc) {
        use core::mem;
        self.disable_cr(rcc);
        mem::forget(self);
    }
}


impl Drop for Rng {

    /// PANICS EVERYTIME! Use .disable(rcc) explicitly!
    fn drop(&mut self) {
        panic!("Use .disable() method on your random struct!");
    }
}
