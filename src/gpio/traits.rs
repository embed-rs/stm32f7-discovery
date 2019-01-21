use super::PinNumber;
use stm32f7::stm32f7x6::{gpioa, gpiob, gpiod};

/// Abstracts over the three different types of input data registers (IDRs).
///
/// The GPIO ports A and B have separate IDR types because they use slightly different
/// reset values. This trait allows to create generic functions that work with all three
/// IDR types.
pub trait IdrTrait {
    /// Represents an IDR value.
    type R: IdrR;

    /// Read the current value from the IDR register.
    fn read(&self) -> Self::R;
}

/// Represents an IDR value that specifies the input value of all pins of the port.
pub trait IdrR {
    /// Returns the input value of the specified pin.
    fn get(&self, pin: PinNumber) -> bool;
}

/// Abstracts over the three different types of output data registers (ODRs).
///
/// The GPIO ports A and B have separate ODR types because they use slightly different
/// reset values. This trait allows to create generic functions that work with all three
/// ODR types.
pub trait OdrTrait {
    /// Represents an ODR value.
    type R: OdrR;

    /// Read the current value from the ODR register.
    fn read(&self) -> Self::R;
}

/// Represents an ODR value that specifies the output value of all pins of the port.
pub trait OdrR {
    /// Returns the output value of the specified pin.
    fn get(&self, pin: PinNumber) -> bool;
}

/// Abstracts over the three different types of bit set and reset registers (BSRRs).
///
/// The GPIO ports A and B have separate BSRR types because they use slightly different
/// reset values. This trait allows to create generic functions that work with all three
/// BSRR types.
pub trait BsrrTrait {
    /// A type that allows BSRR write operations.
    type W: BsrrW;

    /// Perform write operations on the BSRR register.
    fn write<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self::W) -> &mut Self::W;
}

/// Allows writing the BSRR register.
///
/// Bits that are neither `set` or `reset` keep their previous value.
pub trait BsrrW {
    /// Set the output value of the specified pin to 1.
    fn set(&mut self, pin: PinNumber) -> &mut Self;

    /// Set the output value of the specified pin to 0.
    fn reset(&mut self, pin: PinNumber) -> &mut Self;
}

macro_rules! impl_traits_for {
    ($gpio:tt) => {
        impl IdrTrait for $gpio::IDR {
            type R = $gpio::idr::R;

            fn read(&self) -> Self::R {
                $gpio::IDR::read(self)
            }
        }

        impl IdrR for $gpio::idr::R {
            fn get(&self, pin: PinNumber) -> bool {
                use super::PinNumber::*;
                match pin {
                    Pin0 => self.idr0().bit_is_set(),
                    Pin1 => self.idr1().bit_is_set(),
                    Pin2 => self.idr2().bit_is_set(),
                    Pin3 => self.idr3().bit_is_set(),
                    Pin4 => self.idr4().bit_is_set(),
                    Pin5 => self.idr5().bit_is_set(),
                    Pin6 => self.idr6().bit_is_set(),
                    Pin7 => self.idr7().bit_is_set(),
                    Pin8 => self.idr8().bit_is_set(),
                    Pin9 => self.idr9().bit_is_set(),
                    Pin10 => self.idr10().bit_is_set(),
                    Pin11 => self.idr11().bit_is_set(),
                    Pin12 => self.idr12().bit_is_set(),
                    Pin13 => self.idr13().bit_is_set(),
                    Pin14 => self.idr14().bit_is_set(),
                    Pin15 => self.idr15().bit_is_set(),
                }
            }
        }

        impl OdrTrait for $gpio::ODR {
            type R = $gpio::odr::R;

            fn read(&self) -> Self::R {
                $gpio::ODR::read(self)
            }
        }

        impl OdrR for $gpio::odr::R {
            fn get(&self, pin: PinNumber) -> bool {
                use super::PinNumber::*;
                match pin {
                    Pin0 => self.odr0().bit_is_set(),
                    Pin1 => self.odr1().bit_is_set(),
                    Pin2 => self.odr2().bit_is_set(),
                    Pin3 => self.odr3().bit_is_set(),
                    Pin4 => self.odr4().bit_is_set(),
                    Pin5 => self.odr5().bit_is_set(),
                    Pin6 => self.odr6().bit_is_set(),
                    Pin7 => self.odr7().bit_is_set(),
                    Pin8 => self.odr8().bit_is_set(),
                    Pin9 => self.odr9().bit_is_set(),
                    Pin10 => self.odr10().bit_is_set(),
                    Pin11 => self.odr11().bit_is_set(),
                    Pin12 => self.odr12().bit_is_set(),
                    Pin13 => self.odr13().bit_is_set(),
                    Pin14 => self.odr14().bit_is_set(),
                    Pin15 => self.odr15().bit_is_set(),
                }
            }
        }

        impl BsrrTrait for $gpio::BSRR {
            type W = $gpio::bsrr::W;

            fn write<F>(&mut self, f: F)
            where
                F: FnOnce(&mut Self::W) -> &mut Self::W,
            {
                $gpio::BSRR::write(self, f)
            }
        }

        impl BsrrW for $gpio::bsrr::W {
            fn set(&mut self, pin: PinNumber) -> &mut Self {
                use super::PinNumber::*;
                match pin {
                    Pin0 => self.bs0().set(),
                    Pin1 => self.bs1().set(),
                    Pin2 => self.bs2().set(),
                    Pin3 => self.bs3().set(),
                    Pin4 => self.bs4().set(),
                    Pin5 => self.bs5().set(),
                    Pin6 => self.bs6().set(),
                    Pin7 => self.bs7().set(),
                    Pin8 => self.bs8().set(),
                    Pin9 => self.bs9().set(),
                    Pin10 => self.bs10().set(),
                    Pin11 => self.bs11().set(),
                    Pin12 => self.bs12().set(),
                    Pin13 => self.bs13().set(),
                    Pin14 => self.bs14().set(),
                    Pin15 => self.bs15().set(),
                }
            }

            fn reset(&mut self, pin: PinNumber) -> &mut Self {
                use super::PinNumber::*;
                match pin {
                    Pin0 => self.br0().reset(),
                    Pin1 => self.br1().reset(),
                    Pin2 => self.br2().reset(),
                    Pin3 => self.br3().reset(),
                    Pin4 => self.br4().reset(),
                    Pin5 => self.br5().reset(),
                    Pin6 => self.br6().reset(),
                    Pin7 => self.br7().reset(),
                    Pin8 => self.br8().reset(),
                    Pin9 => self.br9().reset(),
                    Pin10 => self.br10().reset(),
                    Pin11 => self.br11().reset(),
                    Pin12 => self.br12().reset(),
                    Pin13 => self.br13().reset(),
                    Pin14 => self.br14().reset(),
                    Pin15 => self.br15().reset(),
                }
            }
        }
    };
}

impl_traits_for!(gpioa);
impl_traits_for!(gpiob);
impl_traits_for!(gpiod);
