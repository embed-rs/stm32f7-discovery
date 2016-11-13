#![allow(dead_code)]

use volatile::{ReadOnly, WriteOnly, ReadWrite};
use svd_board::gpiod;
use core::marker::PhantomData;
use super::pin::{self, GpioPin, PinNumber, GpioRead, GpioWrite};

pub struct GpioPort<Port: PortNumber> {
    mode: &'static mut ReadWrite<gpiod::Moder>,
    out_type: &'static mut ReadWrite<gpiod::Otyper>,
    out_speed: &'static mut ReadWrite<gpiod::Ospeedr>,
    pupd: &'static mut ReadWrite<gpiod::Pupdr>,
    afrl: &'static mut ReadWrite<gpiod::Afrl>,
    afrh: &'static mut ReadWrite<gpiod::Afrh>,
    idr: &'static ReadOnly<gpiod::Idr>,
    odr: &'static ReadWrite<gpiod::Odr>,
    bsrr: *mut WriteOnly<gpiod::Bsrr>,
    phantom: PhantomData<Port>,
}

pub fn new_port<Port: PortNumber>(mode: &'static mut ReadWrite<gpiod::Moder>,
                                  out_type: &'static mut ReadWrite<gpiod::Otyper>,
                                  out_speed: &'static mut ReadWrite<gpiod::Ospeedr>,
                                  pupd: &'static mut ReadWrite<gpiod::Pupdr>,
                                  afrl: &'static mut ReadWrite<gpiod::Afrl>,
                                  afrh: &'static mut ReadWrite<gpiod::Afrh>,
                                  idr: &'static ReadOnly<gpiod::Idr>,
                                  odr: &'static ReadWrite<gpiod::Odr>,
                                  bsrr: *mut WriteOnly<gpiod::Bsrr>)
                                  -> GpioPort<Port> {
    GpioPort {
        mode: mode,
        out_type: out_type,
        out_speed: out_speed,
        pupd: pupd,
        afrl: afrl,
        afrh: afrh,
        idr: idr,
        odr: odr,
        bsrr: bsrr,
        phantom: PhantomData,
    }
}

impl<Port: PortNumber> GpioPort<Port> {
    pub fn to_input<Pin: PinNumber>(&mut self,
                                    mut pin: GpioPin<Port, Pin>,
                                    resistor: Resistor)
                                    -> GpioRead<Pin> {
        self.set_resistor(&mut pin, resistor);
        self.set_mode(&mut pin, Mode::Input);

        pin::new_read(self.idr)
    }

    pub fn to_output<Pin: PinNumber>(&mut self,
                                     mut pin: GpioPin<Port, Pin>,
                                     typ: Type,
                                     speed: Speed,
                                     resistor: Resistor)
                                     -> GpioWrite<Pin> {

        self.set_resistor(&mut pin, resistor);
        self.set_out_type(&mut pin, typ);
        self.set_out_speed(&mut pin, speed);
        self.set_mode(&mut pin, Mode::Output);

        pin::new_write(self.odr, self.bsrr)
    }

    pub fn to_alternate_function<Pin: PinNumber>(&mut self,
                                                 mut pin: GpioPin<Port, Pin>,
                                                 typ: Type,
                                                 speed: Speed,
                                                 alternate_fn: AlternateFunction,
                                                 resistor: Resistor) {

        self.set_resistor(&mut pin, resistor);
        self.set_out_type(&mut pin, typ);
        self.set_out_speed(&mut pin, speed);
        self.set_alternate_function(&mut pin, alternate_fn);
        self.set_mode(&mut pin, Mode::AlternateFunction);
    }

    fn set_mode<Pin: PinNumber>(&mut self, _pin: &mut GpioPin<Port, Pin>, mode: Mode) {
        self.mode.update(|r| Pin::set_mode(r, mode as u8));
    }

    fn set_resistor<Pin: PinNumber>(&mut self, _pin: &mut GpioPin<Port, Pin>, resistor: Resistor) {
        self.pupd.update(|r| Pin::set_pupd(r, resistor as u8));
    }

    fn set_out_type<Pin: PinNumber>(&mut self, _pin: &mut GpioPin<Port, Pin>, out_type: Type) {
        let value = match out_type {
            Type::PushPull => false,
            Type::OpenDrain => true,
        };
        self.out_type.update(|r| Pin::set_type(r, value));
    }

    fn set_out_speed<Pin: PinNumber>(&mut self, _pin: &mut GpioPin<Port, Pin>, out_speed: Speed) {
        self.out_speed.update(|r| Pin::set_speed(r, out_speed as u8));
    }

    fn set_alternate_function<Pin: PinNumber>(&mut self,
                                              _pin: &mut GpioPin<Port, Pin>,
                                              alternate_fn: AlternateFunction) {

        Pin::set_alternate_fn(self.afrl, self.afrh, alternate_fn as u8);
    }
}

pub trait PortNumber {}

pub enum PortA {}
pub enum PortB {}
pub enum PortC {}
pub enum PortD {}
pub enum PortE {}
pub enum PortF {}
pub enum PortG {}
pub enum PortH {}
pub enum PortI {}
pub enum PortJ {}
pub enum PortK {}

impl PortNumber for PortA {}
impl PortNumber for PortB {}
impl PortNumber for PortC {}
impl PortNumber for PortD {}
impl PortNumber for PortE {}
impl PortNumber for PortF {}
impl PortNumber for PortG {}
impl PortNumber for PortH {}
impl PortNumber for PortI {}
impl PortNumber for PortJ {}
impl PortNumber for PortK {}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Resistor {
    NoPull = 0b00,
    PullUp = 0b01,
    PullDown = 0b10,
}

#[derive(Debug, Clone, Copy)]
pub enum Type {
    PushPull = 0,
    OpenDrain = 1,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Speed {
    Low = 0b00,
    Medium = 0b01,
    High = 0b10,
    VeryHigh = 0b11,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AlternateFunction {
    AF0 = 0b0000,
    AF1 = 0b0001,
    AF2 = 0b0010,
    AF3 = 0b0011,
    AF4 = 0b0100,
    AF5 = 0b0101,
    AF6 = 0b0110,
    AF7 = 0b0111,
    AF8 = 0b1000,
    AF9 = 0b1001,
    AF10 = 0b1010,
    AF11 = 0b1011,
    AF12 = 0b1100,
    AF13 = 0b1101,
    AF14 = 0b1110,
    AF15 = 0b1111,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum Mode {
    Input = 0b00,
    Output = 0b01,
    AlternateFunction = 0b10,
    Analog = 0b11,
}
