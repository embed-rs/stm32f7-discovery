#![allow(dead_code)]

pub use self::port::{GpioPort, PortNumber, Type, Speed, Resistor, AlternateFunction};
pub use self::port::{PortA, PortB, PortC, PortD, PortE, PortF, PortG, PortH, PortI, PortJ, PortK};
pub use self::pin::{GpioPin, PinNumber, GpioWrite, GpioRead};
pub use self::pin::{Pin0, Pin1, Pin2, Pin3, Pin4, Pin5, Pin6, Pin7, Pin8, Pin9, Pin10, Pin11};
pub use self::pin::{Pin12, Pin13, Pin14, Pin15};

use svd_board;
use svd_board::gpiod::Gpiod;
use core::marker::PhantomData;

mod port;
mod pin;

pub struct GpioController {
    port_a: GpioPort<PortA>,
    port_b: GpioPort<PortB>,
    port_c: GpioPort<PortC>,
    port_d: GpioPort<PortD>,
    port_e: GpioPort<PortE>,
    port_f: GpioPort<PortF>,
    port_g: GpioPort<PortG>,
    port_h: GpioPort<PortH>,
    port_i: GpioPort<PortI>,
    port_j: GpioPort<PortJ>,
    port_k: GpioPort<PortK>,
    pub pins: Pins,
}

impl GpioController {
    pub unsafe fn new(a: &'static mut svd_board::gpioa::Gpioa,
                      b: &'static mut svd_board::gpiob::Gpiob,
                      c: &'static mut svd_board::gpioc::Gpioc,
                      d: &'static mut svd_board::gpiod::Gpiod,
                      e: &'static mut svd_board::gpioe::Gpioe,
                      f: &'static mut svd_board::gpiof::Gpiof,
                      g: &'static mut svd_board::gpiog::Gpiog,
                      h: &'static mut svd_board::gpioh::Gpioh,
                      i: &'static mut svd_board::gpioi::Gpioi,
                      j: &'static mut svd_board::gpioj::Gpioj,
                      k: &'static mut svd_board::gpiok::Gpiok)
                      -> GpioController {
        let (a, a_pins) = Gpio::new(&mut *(a as *mut _ as *mut _)).split();
        let (b, b_pins) = Gpio::new(&mut *(b as *mut _ as *mut _)).split();
        let (c, c_pins) = Gpio::new(c).split();
        let (d, d_pins) = Gpio::new(d).split();
        let (e, e_pins) = Gpio::new(e).split();
        let (f, f_pins) = Gpio::new(f).split();
        let (g, g_pins) = Gpio::new(g).split();
        let (h, h_pins) = Gpio::new(h).split();
        let (i, i_pins) = Gpio::new(i).split();
        let (j, j_pins) = Gpio::new(j).split();
        let (k, k_pins) = Gpio::new(k).split();

        let pins = Pins {
            a: PortPins::from(a_pins),
            b: PortPins::from(b_pins),
            c: PortPins::from(c_pins),
            d: PortPins::from(d_pins),
            e: PortPins::from(e_pins),
            f: PortPins::from(f_pins),
            g: PortPins::from(g_pins),
            h: PortPins::from(h_pins),
            i: PortPins::from(i_pins),
            j: PortPins::from(j_pins),
            k: PortPins::from(k_pins),
        };

        GpioController {
            port_a: a,
            port_b: b,
            port_c: c,
            port_d: d,
            port_e: e,
            port_f: f,
            port_g: g,
            port_h: h,
            port_i: i,
            port_j: j,
            port_k: k,
            pins: pins,
        }
    }

    pub fn to_input<Port, Pin>(&mut self,
                               pin: GpioPin<Port, Pin>,
                               resistor: Resistor)
                               -> GpioRead<Pin>
        where Self: PortDeref<Port>,
              Port: PortNumber,
              Pin: PinNumber
    {
        self.port().to_input(pin, resistor)
    }

    pub fn to_output<Port, Pin>(&mut self,
                                pin: GpioPin<Port, Pin>,
                                typ: Type,
                                speed: Speed,
                                resistor: Resistor)
                                -> GpioWrite<Pin>
        where Self: PortDeref<Port>,
              Port: PortNumber,
              Pin: PinNumber
    {
        self.port().to_output(pin, typ, speed, resistor)
    }

    pub fn to_alternate_function<Port, Pin>(&mut self,
                                            pin: GpioPin<Port, Pin>,
                                            typ: Type,
                                            speed: Speed,
                                            alternate_fn: AlternateFunction,
                                            resistor: Resistor)
        where Self: PortDeref<Port>,
              Port: PortNumber,
              Pin: PinNumber
    {
        self.port().to_alternate_function(pin, typ, speed, alternate_fn, resistor);
    }
}

pub trait PortDeref<P: PortNumber> {
    fn port(&mut self) -> &mut GpioPort<P>;
}

impl PortDeref<PortA> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortA> {
        &mut self.port_a
    }
}

impl PortDeref<PortB> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortB> {
        &mut self.port_b
    }
}

impl PortDeref<PortC> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortC> {
        &mut self.port_c
    }
}

impl PortDeref<PortD> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortD> {
        &mut self.port_d
    }
}

impl PortDeref<PortE> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortE> {
        &mut self.port_e
    }
}

impl PortDeref<PortF> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortF> {
        &mut self.port_f
    }
}

impl PortDeref<PortG> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortG> {
        &mut self.port_g
    }
}

impl PortDeref<PortH> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortH> {
        &mut self.port_h
    }
}

impl PortDeref<PortI> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortI> {
        &mut self.port_i
    }
}

impl PortDeref<PortJ> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortJ> {
        &mut self.port_j
    }
}

impl PortDeref<PortK> for GpioController {
    fn port(&mut self) -> &mut GpioPort<PortK> {
        &mut self.port_k
    }
}

pub struct Pins {
    pub a: PortPins<PortA>,
    pub b: PortPins<PortB>,
    pub c: PortPins<PortC>,
    pub d: PortPins<PortD>,
    pub e: PortPins<PortE>,
    pub f: PortPins<PortF>,
    pub g: PortPins<PortG>,
    pub h: PortPins<PortH>,
    pub i: PortPins<PortI>,
    pub j: PortPins<PortJ>,
    pub k: PortPins<PortK>,
}

pub struct PortPins<Port: PortNumber>(pub Option<GpioPin<Port, Pin0>>,
                                      pub Option<GpioPin<Port, Pin1>>,
                                      pub Option<GpioPin<Port, Pin2>>,
                                      pub Option<GpioPin<Port, Pin3>>,
                                      pub Option<GpioPin<Port, Pin4>>,
                                      pub Option<GpioPin<Port, Pin5>>,
                                      pub Option<GpioPin<Port, Pin6>>,
                                      pub Option<GpioPin<Port, Pin7>>,
                                      pub Option<GpioPin<Port, Pin8>>,
                                      pub Option<GpioPin<Port, Pin9>>,
                                      pub Option<GpioPin<Port, Pin10>>,
                                      pub Option<GpioPin<Port, Pin11>>,
                                      pub Option<GpioPin<Port, Pin12>>,
                                      pub Option<GpioPin<Port, Pin13>>,
                                      pub Option<GpioPin<Port, Pin14>>,
                                      pub Option<GpioPin<Port, Pin15>>);

impl<Port: PortNumber> From<PortPinsAll<Port>> for PortPins<Port> {
    fn from(pins: PortPinsAll<Port>) -> PortPins<Port> {
        PortPins(Some(pins.0),
                 Some(pins.1),
                 Some(pins.2),
                 Some(pins.3),
                 Some(pins.4),
                 Some(pins.5),
                 Some(pins.6),
                 Some(pins.7),
                 Some(pins.8),
                 Some(pins.9),
                 Some(pins.10),
                 Some(pins.11),
                 Some(pins.12),
                 Some(pins.13),
                 Some(pins.14),
                 Some(pins.15))
    }
}

pub struct Gpio<Port: PortNumber> {
    registers: &'static mut Gpiod,
    phantom: PhantomData<Port>,
}

impl<Port: PortNumber> Gpio<Port> {
    /// Safety: It's unsafe to create two Gpios with the same port number.
    pub unsafe fn new(registers: &'static mut Gpiod) -> Gpio<Port> {
        Gpio {
            registers: registers,
            phantom: PhantomData,
        }
    }

    pub fn split(self) -> (GpioPort<Port>, PortPinsAll<Port>) {
        let &mut Gpiod { ref mut moder,
                         ref mut otyper,
                         ref mut ospeedr,
                         ref mut pupdr,
                         ref idr,
                         ref odr,
                         ref mut bsrr,
                         ref mut afrl,
                         ref mut afrh,
                         .. } = self.registers;

        let bank_ref = port::new_port(moder, otyper, ospeedr, pupdr, afrl, afrh, idr, odr, bsrr);

        let pins = PortPinsAll(pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin(),
                               pin::new_pin());

        (bank_ref, pins)
    }
}

pub struct PortPinsAll<Port: PortNumber>(GpioPin<Port, Pin0>,
                                         GpioPin<Port, Pin1>,
                                         GpioPin<Port, Pin2>,
                                         GpioPin<Port, Pin3>,
                                         GpioPin<Port, Pin4>,
                                         GpioPin<Port, Pin5>,
                                         GpioPin<Port, Pin6>,
                                         GpioPin<Port, Pin7>,
                                         GpioPin<Port, Pin8>,
                                         GpioPin<Port, Pin9>,
                                         GpioPin<Port, Pin10>,
                                         GpioPin<Port, Pin11>,
                                         GpioPin<Port, Pin12>,
                                         GpioPin<Port, Pin13>,
                                         GpioPin<Port, Pin14>,
                                         GpioPin<Port, Pin15>);
