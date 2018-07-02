use core::marker::PhantomData;

pub use self::port::*;
pub use self::traits::*;

mod port;
mod traits;

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

pub struct InputPin<'a, IDR: IdrTrait + 'a> {
    pin: PinNumber,
    input_data: &'a IDR,
}

impl<'a, IDR> InputPin<'a, IDR>
where
    IDR: IdrTrait,
{
    pub fn get(&self) -> bool {
        let value = self.input_data.read();
        value.get(self.pin)
    }
}

pub trait OutputPin: Sized {
    fn get(&self) -> bool;

    fn set(&mut self, value: bool);

    fn toggle(&mut self) {
        let current = self.get();
        self.set(!current);
    }
}

pub struct OutputPinImpl<'a, ODR: OdrTrait + 'a, BSRR: BsrrTrait + 'a> {
    pin: PinNumber,
    output_data: &'a ODR,
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
        unsafe { (&mut *self.register) }.write(|w| {
            if value {
                w.set(pin)
            } else {
                w.reset(pin)
            }
        });
    }
}
