use core::marker::PhantomData;
use stm32f7::stm32f7x6::{gpioa, gpiob, gpiod};

pub use self::port::*;

mod port;

#[derive(Debug)]
pub enum Error {
    PinAlreadyInUse(PinNumber),
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Input,
    Output,
    Alternate,
    Analog,
}

#[derive(Debug, Clone, Copy)]
pub enum Resistor {
    NoPull,
    PullUp,
    PullDown,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputType {
    PushPull,
    OpenDrain,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputSpeed {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Copy)]
pub enum AlternateFunction {
    AF0,
    AF1,
    AF2,
    AF3,
    AF4,
    AF5,
    AF6,
    AF7,
    AF8,
    AF9,
    AF10,
    AF11,
    AF12,
    AF13,
    AF14,
    AF15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PinNumber {
    Pin0 = 0,
    Pin1,
    Pin2,
    Pin3,
    Pin4,
    Pin5,
    Pin6,
    Pin7,
    Pin8,
    Pin9,
    Pin10,
    Pin11,
    Pin12,
    Pin13,
    Pin14,
    Pin15,
}

pub struct InputPin<'a, IDR: 'a> {
    pin: PinNumber,
    input_data: &'a IDR,
}

macro_rules! impl_input_pin {
    ($x:ty) => {
        impl<'a> InputPin<'a, $x> {
            pub fn get(&self) -> bool {
                use self::PinNumber::*;
                let value = self.input_data.read();
                match self.pin {
                    Pin0 => value.idr0().bit_is_set(),
                    Pin1 => value.idr1().bit_is_set(),
                    Pin2 => value.idr2().bit_is_set(),
                    Pin3 => value.idr3().bit_is_set(),
                    Pin4 => value.idr4().bit_is_set(),
                    Pin5 => value.idr5().bit_is_set(),
                    Pin6 => value.idr6().bit_is_set(),
                    Pin7 => value.idr7().bit_is_set(),
                    Pin8 => value.idr8().bit_is_set(),
                    Pin9 => value.idr9().bit_is_set(),
                    Pin10 => value.idr10().bit_is_set(),
                    Pin11 => value.idr11().bit_is_set(),
                    Pin12 => value.idr12().bit_is_set(),
                    Pin13 => value.idr13().bit_is_set(),
                    Pin14 => value.idr14().bit_is_set(),
                    Pin15 => value.idr15().bit_is_set(),
                }
            }
        }
    };
}

impl_input_pin!(gpioa::IDR);
impl_input_pin!(gpiob::IDR);
impl_input_pin!(gpiod::IDR);

pub struct OutputPin<'a, ODR: 'a, BSRR: 'a> {
    pin: PinNumber,
    output_data: &'a ODR,
    bit_set_reset: BsrrRef<'a, BSRR>,
}

macro_rules! impl_output_pin {
    ($x:ty, $y:ty) => {
        impl<'a> OutputPin<'a, $x, $y> {
            pub fn get(&self) -> bool {
                use self::PinNumber::*;
                let value = self.output_data.read();
                match self.pin {
                    Pin0 => value.odr0().bit_is_set(),
                    Pin1 => value.odr1().bit_is_set(),
                    Pin2 => value.odr2().bit_is_set(),
                    Pin3 => value.odr3().bit_is_set(),
                    Pin4 => value.odr4().bit_is_set(),
                    Pin5 => value.odr5().bit_is_set(),
                    Pin6 => value.odr6().bit_is_set(),
                    Pin7 => value.odr7().bit_is_set(),
                    Pin8 => value.odr8().bit_is_set(),
                    Pin9 => value.odr9().bit_is_set(),
                    Pin10 => value.odr10().bit_is_set(),
                    Pin11 => value.odr11().bit_is_set(),
                    Pin12 => value.odr12().bit_is_set(),
                    Pin13 => value.odr13().bit_is_set(),
                    Pin14 => value.odr14().bit_is_set(),
                    Pin15 => value.odr15().bit_is_set(),
                }
            }

            pub fn set(&mut self, value: bool) {
                self.bit_set_reset.set(self.pin, value);
            }

            pub fn toggle(&mut self) {
                let current = self.get();
                self.set(!current);
            }
        }
    };
}

impl_output_pin!(gpioa::ODR, gpioa::BSRR);
impl_output_pin!(gpiob::ODR, gpiob::BSRR);
impl_output_pin!(gpiod::ODR, gpiod::BSRR);

#[derive(Debug, Clone)]
struct BsrrRef<'a, BSRR: 'a> {
    register: *mut BSRR,
    phantom: PhantomData<&'a BSRR>,
}

unsafe impl<'a, U> Send for BsrrRef<'a, U> {}

macro_rules! impl_bssr_ref {
    ($x:ty) => {
        impl<'a> BsrrRef<'a, $x> {
            fn set(&self, pin: PinNumber, value: bool) {
                use self::PinNumber::*;

                unsafe { (&mut *self.register) }.write(|w| {
                    if value {
                        // set the bit
                        match pin {
                            Pin0 => w.bs0().set(),
                            Pin1 => w.bs1().set(),
                            Pin2 => w.bs2().set(),
                            Pin3 => w.bs3().set(),
                            Pin4 => w.bs4().set(),
                            Pin5 => w.bs5().set(),
                            Pin6 => w.bs6().set(),
                            Pin7 => w.bs7().set(),
                            Pin8 => w.bs8().set(),
                            Pin9 => w.bs9().set(),
                            Pin10 => w.bs10().set(),
                            Pin11 => w.bs11().set(),
                            Pin12 => w.bs12().set(),
                            Pin13 => w.bs13().set(),
                            Pin14 => w.bs14().set(),
                            Pin15 => w.bs15().set(),
                        }
                    } else {
                        // reset the bit
                        match pin {
                            Pin0 => w.br0().reset(),
                            Pin1 => w.br1().reset(),
                            Pin2 => w.br2().reset(),
                            Pin3 => w.br3().reset(),
                            Pin4 => w.br4().reset(),
                            Pin5 => w.br5().reset(),
                            Pin6 => w.br6().reset(),
                            Pin7 => w.br7().reset(),
                            Pin8 => w.br8().reset(),
                            Pin9 => w.br9().reset(),
                            Pin10 => w.br10().reset(),
                            Pin11 => w.br11().reset(),
                            Pin12 => w.br12().reset(),
                            Pin13 => w.br13().reset(),
                            Pin14 => w.br14().reset(),
                            Pin15 => w.br15().reset(),
                        }
                    }
                });
            }
        }
    };
}

impl_bssr_ref!(gpioa::BSRR);
impl_bssr_ref!(gpiob::BSRR);
impl_bssr_ref!(gpiod::BSRR);
