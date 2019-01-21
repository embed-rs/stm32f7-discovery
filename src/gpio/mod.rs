//! Abstractions for GPIO ports.

use core::marker::PhantomData;

pub use self::port::*;
pub use self::traits::*;

mod port;
mod traits;

/// The different possible modes of a GPIO pin.
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    /// Use the pin for receiving data.
    Input,
    /// Use the pin for sending data.
    Output,
    /// Activate an alternate function of the pin to make it usable to some connected device.
    Alternate,
    /// Use the pin in analog mode.
    Analog,
}

/// Pull the pin value up or down.
#[derive(Debug, Clone, Copy)]
pub enum Resistor {
    /// Don't pull the value.
    NoPull,
    /// Pull the value to 1 if no data is sent/received.
    PullUp,
    /// Pull the value to 0 if no data is sent/received.
    PullDown,
}

/// The output mode of the pin.
#[derive(Debug, Clone, Copy)]
pub enum OutputType {
    /// Use push-pull mode.
    PushPull,
    /// Use open drain mode.
    OpenDrain,
}

/// The different output speeds.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy)]
pub enum OutputSpeed {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// The possible alternate functions.
///
/// The alternate function number that a device uses is specified in the manual.
#[allow(missing_docs)]
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

/// The 16 possible pin numbers.
#[allow(missing_docs)]
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

/// High level abstraction of a GPIO pin configured as input.
pub trait InputPin: Sized {
    /// Get the current input value of the pin.
    fn get(&self) -> bool;
}

/// An implementation of the `InputPin` trait for the IDR abstractions of this module.
pub struct InputPinImpl<'a, IDR: IdrTrait + 'a> {
    pin: PinNumber,
    input_data: ReadOnlyIdr<'a, IDR>,
}

impl<'a, IDR> InputPin for InputPinImpl<'a, IDR>
where
    IDR: IdrTrait,
{
    fn get(&self) -> bool {
        let value = self.input_data.read();
        value.get(self.pin)
    }
}

struct ReadOnlyIdr<'a, IDR: IdrTrait>(&'a IDR);

impl<'a, IDR: IdrTrait> ReadOnlyIdr<'a, IDR> {
    fn read(&self) -> IDR::R {
        self.0.read()
    }
}

unsafe impl<'a, IDR: IdrTrait> Sync for ReadOnlyIdr<'a, IDR> {}
unsafe impl<'a, IDR: IdrTrait> Send for ReadOnlyIdr<'a, IDR> {}

/// High level abstraction of a GPIO pin configured as output.
pub trait OutputPin: Sized {
    /// Get the current output value of the pin.
    fn get(&self) -> bool;

    /// Set the output value of the pin.
    fn set(&mut self, value: bool);

    /// Toggle the output value of the pin.
    fn toggle(&mut self) {
        let current = self.get();
        self.set(!current);
    }
}

/// An implementation of the `OutputPin` trait for the ODR and BSRR abstractions of this module.
pub struct OutputPinImpl<'a, ODR: OdrTrait + 'a, BSRR: BsrrTrait + 'a> {
    pin: PinNumber,
    output_data: ReadOnlyOdr<'a, ODR>,
    bit_set_reset: BsrrRef<'a, BSRR>,
}

impl<'a, ODR, BSRR> OutputPin for OutputPinImpl<'a, ODR, BSRR>
where
    ODR: OdrTrait,
    BSRR: BsrrTrait,
{
    fn get(&self) -> bool {
        let value = self.output_data.read();
        value.get(self.pin)
    }

    fn set(&mut self, value: bool) {
        self.bit_set_reset.set(self.pin, value);
    }
}

struct ReadOnlyOdr<'a, ODR: OdrTrait>(&'a ODR);

impl<'a, ODR: OdrTrait> ReadOnlyOdr<'a, ODR> {
    fn read(&self) -> ODR::R {
        self.0.read()
    }
}

unsafe impl<'a, ODR: OdrTrait> Send for ReadOnlyOdr<'a, ODR> {}
unsafe impl<'a, ODR: OdrTrait> Sync for ReadOnlyOdr<'a, ODR> {}

#[derive(Debug, Clone)]
struct BsrrRef<'a, BSRR: 'a> {
    register: *mut BSRR,
    phantom: PhantomData<&'a BSRR>,
}

unsafe impl<'a, U> Send for BsrrRef<'a, U> {}

impl<'a, BSRR> BsrrRef<'a, BSRR>
where
    BSRR: BsrrTrait,
{
    fn set(&self, pin: PinNumber, value: bool) {
        unsafe { (&mut *self.register) }.write(|w| if value { w.set(pin) } else { w.reset(pin) });
    }
}
