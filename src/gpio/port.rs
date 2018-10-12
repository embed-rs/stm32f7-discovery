use super::*;
use core::marker::PhantomData;
use stm32f7::stm32f7x6::{gpioa, gpiob, gpiod};

pub struct GpioPort<T> {
    pub(super) pin_in_use: [bool; 16],
    register_block: T,
}

pub struct RegisterBlock<'a, I: 'a, O: 'a, M: 'a, P: 'a, B: 'a, T: 'a, S: 'a, AH: 'a, AL: 'a> {
    idr: &'a I,
    odr: &'a O,
    moder: &'a M,
    pupdr: &'a P,
    bsrr: &'a B,
    otyper: &'a T,
    ospeedr: &'a S,
    afrh: &'a AH,
    afrl: &'a AL,
}

pub type RegisterBlockA<'a> = RegisterBlock<
    'a,
    gpioa::IDR,
    gpioa::ODR,
    gpioa::MODER,
    gpioa::PUPDR,
    gpioa::BSRR,
    gpioa::OTYPER,
    gpioa::OSPEEDR,
    gpioa::AFRH,
    gpioa::AFRL,
>;
pub type RegisterBlockB<'a> = RegisterBlock<
    'a,
    gpiob::IDR,
    gpiob::ODR,
    gpiob::MODER,
    gpiob::PUPDR,
    gpiob::BSRR,
    gpiob::OTYPER,
    gpiob::OSPEEDR,
    gpiob::AFRH,
    gpiob::AFRL,
>;
pub type RegisterBlockD<'a> = RegisterBlock<
    'a,
    gpiod::IDR,
    gpiod::ODR,
    gpiod::MODER,
    gpiod::PUPDR,
    gpiod::BSRR,
    gpiod::OTYPER,
    gpiod::OSPEEDR,
    gpiod::AFRH,
    gpiod::AFRL,
>;

macro_rules! new_gpio_port {
    ($register_block:expr) => {
        GpioPort {
            pin_in_use: [false; 16],
            register_block: RegisterBlock {
                idr: &$register_block.idr,
                odr: &$register_block.odr,
                moder: &$register_block.moder,
                pupdr: &$register_block.pupdr,
                bsrr: &$register_block.bsrr,
                otyper: &$register_block.otyper,
                ospeedr: &$register_block.ospeedr,
                afrh: &$register_block.afrh,
                afrl: &$register_block.afrl,
            },
        }
    };
}

impl<'a> GpioPort<RegisterBlockA<'a>> {
    pub fn new_a(register_block: &'a gpioa::RegisterBlock) -> Self {
        new_gpio_port!(register_block)
    }
}

impl<'a> GpioPort<RegisterBlockB<'a>> {
    pub fn new_b(register_block: &'a gpiob::RegisterBlock) -> Self {
        new_gpio_port!(register_block)
    }
}

impl<'a> GpioPort<RegisterBlockD<'a>> {
    pub fn new(register_block: &'a gpiod::RegisterBlock) -> Self {
        new_gpio_port!(register_block)
    }
}

pub trait RegisterBlockTrait<'a> {
    type Idr: IdrTrait + 'a;
    type Odr: OdrTrait + 'a;
    type Bsrr: BsrrTrait + 'a;

    fn idr(&self) -> &'a Self::Idr;
    fn odr(&self) -> &'a Self::Odr;
    fn bsrr(&self) -> &'a Self::Bsrr;
    fn set_mode(&mut self, pins: &[PinNumber], mode: Mode);
    fn set_resistor(&mut self, pins: &[PinNumber], resistor: Resistor);
    fn set_out_type(&mut self, pins: &[PinNumber], out_type: OutputType);
    fn set_out_speed(&mut self, pins: &[PinNumber], out_speed: OutputSpeed);
    fn set_alternate_fn(&mut self, pins: &[PinNumber], alternate_fn: AlternateFunction);
}

impl<'a, T: RegisterBlockTrait<'a>> GpioPort<T> {
    pub fn to_input(
        &mut self,
        pin: PinNumber,
        resistor: Resistor,
    ) -> Result<impl InputPin + 'a, Error> {
        self.use_pin(pin)?;

        self.register_block.set_mode(&[pin], Mode::Input);
        self.register_block.set_resistor(&[pin], resistor);

        Ok(InputPinImpl {
            pin: pin,
            input_data: self.register_block.idr(),
        })
    }

    pub fn to_output(
        &mut self,
        pin: PinNumber,
        out_type: OutputType,
        out_speed: OutputSpeed,
        resistor: Resistor,
    ) -> Result<impl OutputPin + 'a, Error> {
        self.use_pin(pin)?;

        self.register_block.set_mode(&[pin], Mode::Output);
        self.register_block.set_out_type(&[pin], out_type);
        self.register_block.set_out_speed(&[pin], out_speed);
        self.register_block.set_resistor(&[pin], resistor);

        let output_pin: OutputPinImpl<T::Odr, T::Bsrr> = OutputPinImpl {
            pin: pin,
            output_data: self.register_block.odr(),
            bit_set_reset: BsrrRef {
                register: self.register_block.bsrr() as *const _ as *mut _,
                phantom: PhantomData,
            },
        };
        Ok(output_pin)
    }

    pub fn to_alternate_function(
        &mut self,
        pin: PinNumber,
        alternate_fn: AlternateFunction,
        typ: OutputType,
        speed: OutputSpeed,
        resistor: Resistor,
    ) -> Result<(), Error> {
        self.to_alternate_function_all(&[pin], alternate_fn, typ, speed, resistor)
    }

    pub fn to_alternate_function_all(
        &mut self,
        pins: &[PinNumber],
        alternate_fn: AlternateFunction,
        typ: OutputType,
        speed: OutputSpeed,
        resistor: Resistor,
    ) -> Result<(), Error> {
        self.use_pins(pins)?;

        self.register_block.set_mode(pins, Mode::Alternate);
        self.register_block.set_resistor(pins, resistor);
        self.register_block.set_out_type(pins, typ);
        self.register_block.set_out_speed(pins, speed);
        self.register_block.set_alternate_fn(pins, alternate_fn);

        Ok(())
    }

    fn use_pin(&mut self, pin: PinNumber) -> Result<(), Error> {
        if self.pin_in_use[pin as usize] {
            Err(Error::PinAlreadyInUse(pin))
        } else {
            self.pin_in_use[pin as usize] = true;
            Ok(())
        }
    }

    fn use_pins(&mut self, pins: &[PinNumber]) -> Result<(), Error> {
        // create a copy of the pin_in_use array since we only want to modify it in case of success
        let mut pin_in_use = self.pin_in_use;

        for &pin in pins {
            if pin_in_use[pin as usize] {
                return Err(Error::PinAlreadyInUse(pin));
            } else {
                pin_in_use[pin as usize] = true;
            }
        }

        // success => write back updated pin_in_use array
        self.pin_in_use = pin_in_use;

        Ok(())
    }
}

macro_rules! impl_register_block_trait {
    ($register_block:tt, $gpio:tt) => {
        impl<'a> RegisterBlockTrait<'a> for $register_block<'a> {
            type Idr = $gpio::IDR;
            type Odr = $gpio::ODR;
            type Bsrr = $gpio::BSRR;

            fn idr(&self) -> &'a Self::Idr {
                self.idr
            }

            fn odr(&self) -> &'a Self::Odr {
                self.odr
            }

            fn bsrr(&self) -> &'a Self::Bsrr {
                self.bsrr
            }

            fn set_mode(&mut self, pins: &[PinNumber], mode: Mode) {
                use self::PinNumber::*;
                use stm32f7::stm32f7x6::$gpio::moder::MODER15W;

                let variant = || match mode {
                    Mode::Input => MODER15W::INPUT,
                    Mode::Output => MODER15W::OUTPUT,
                    Mode::Alternate => MODER15W::ALTERNATE,
                    Mode::Analog => MODER15W::ANALOG,
                };

                self.moder.modify(|_, w| {
                    for pin in pins {
                        match pin {
                            Pin0 => w.moder0().variant(variant()),
                            Pin1 => w.moder1().variant(variant()),
                            Pin2 => w.moder2().variant(variant()),
                            Pin3 => w.moder3().variant(variant()),
                            Pin4 => w.moder4().variant(variant()),
                            Pin5 => w.moder5().variant(variant()),
                            Pin6 => w.moder6().variant(variant()),
                            Pin7 => w.moder7().variant(variant()),
                            Pin8 => w.moder8().variant(variant()),
                            Pin9 => w.moder9().variant(variant()),
                            Pin10 => w.moder10().variant(variant()),
                            Pin11 => w.moder11().variant(variant()),
                            Pin12 => w.moder12().variant(variant()),
                            Pin13 => w.moder13().variant(variant()),
                            Pin14 => w.moder14().variant(variant()),
                            Pin15 => w.moder15().variant(variant()),
                        };
                    }
                    w
                })
            }

            fn set_resistor(&mut self, pins: &[PinNumber], resistor: Resistor) {
                use self::PinNumber::*;
                use stm32f7::stm32f7x6::$gpio::pupdr::PUPDR15W;

                let variant = || match resistor {
                    Resistor::NoPull => PUPDR15W::FLOATING,
                    Resistor::PullUp => PUPDR15W::PULLUP,
                    Resistor::PullDown => PUPDR15W::PULLDOWN,
                };

                self.pupdr.modify(|_, w| {
                    for pin in pins {
                        match pin {
                            Pin0 => w.pupdr0().variant(variant()),
                            Pin1 => w.pupdr1().variant(variant()),
                            Pin2 => w.pupdr2().variant(variant()),
                            Pin3 => w.pupdr3().variant(variant()),
                            Pin4 => w.pupdr4().variant(variant()),
                            Pin5 => w.pupdr5().variant(variant()),
                            Pin6 => w.pupdr6().variant(variant()),
                            Pin7 => w.pupdr7().variant(variant()),
                            Pin8 => w.pupdr8().variant(variant()),
                            Pin9 => w.pupdr9().variant(variant()),
                            Pin10 => w.pupdr10().variant(variant()),
                            Pin11 => w.pupdr11().variant(variant()),
                            Pin12 => w.pupdr12().variant(variant()),
                            Pin13 => w.pupdr13().variant(variant()),
                            Pin14 => w.pupdr14().variant(variant()),
                            Pin15 => w.pupdr15().variant(variant()),
                        };
                    }
                    w
                });
            }

            fn set_out_type(&mut self, pins: &[PinNumber], out_type: OutputType) {
                use self::PinNumber::*;
                use stm32f7::stm32f7x6::$gpio::otyper::OT15W;

                let variant = || match out_type {
                    OutputType::OpenDrain => OT15W::OPENDRAIN,
                    OutputType::PushPull => OT15W::PUSHPULL,
                };

                self.otyper.modify(|_, w| {
                    for pin in pins {
                        match pin {
                            Pin0 => w.ot0().variant(variant()),
                            Pin1 => w.ot1().variant(variant()),
                            Pin2 => w.ot2().variant(variant()),
                            Pin3 => w.ot3().variant(variant()),
                            Pin4 => w.ot4().variant(variant()),
                            Pin5 => w.ot5().variant(variant()),
                            Pin6 => w.ot6().variant(variant()),
                            Pin7 => w.ot7().variant(variant()),
                            Pin8 => w.ot8().variant(variant()),
                            Pin9 => w.ot9().variant(variant()),
                            Pin10 => w.ot10().variant(variant()),
                            Pin11 => w.ot11().variant(variant()),
                            Pin12 => w.ot12().variant(variant()),
                            Pin13 => w.ot13().variant(variant()),
                            Pin14 => w.ot14().variant(variant()),
                            Pin15 => w.ot15().variant(variant()),
                        };
                    }
                    w
                })
            }

            fn set_out_speed(&mut self, pins: &[PinNumber], out_speed: OutputSpeed) {
                use self::PinNumber::*;
                use stm32f7::stm32f7x6::$gpio::ospeedr::OSPEEDR15W;

                let variant = || match out_speed {
                    OutputSpeed::Low => OSPEEDR15W::LOWSPEED,
                    OutputSpeed::Medium => OSPEEDR15W::MEDIUMSPEED,
                    OutputSpeed::High => OSPEEDR15W::HIGHSPEED,
                    OutputSpeed::VeryHigh => OSPEEDR15W::VERYHIGHSPEED,
                };

                self.ospeedr.modify(|_, w| {
                    for pin in pins {
                        match pin {
                            Pin0 => w.ospeedr0().variant(variant()),
                            Pin1 => w.ospeedr1().variant(variant()),
                            Pin2 => w.ospeedr2().variant(variant()),
                            Pin3 => w.ospeedr3().variant(variant()),
                            Pin4 => w.ospeedr4().variant(variant()),
                            Pin5 => w.ospeedr5().variant(variant()),
                            Pin6 => w.ospeedr6().variant(variant()),
                            Pin7 => w.ospeedr7().variant(variant()),
                            Pin8 => w.ospeedr8().variant(variant()),
                            Pin9 => w.ospeedr9().variant(variant()),
                            Pin10 => w.ospeedr10().variant(variant()),
                            Pin11 => w.ospeedr11().variant(variant()),
                            Pin12 => w.ospeedr12().variant(variant()),
                            Pin13 => w.ospeedr13().variant(variant()),
                            Pin14 => w.ospeedr14().variant(variant()),
                            Pin15 => w.ospeedr15().variant(variant()),
                        };
                    }
                    w
                })
            }

            fn set_alternate_fn(&mut self, pins: &[PinNumber], alternate_fn: AlternateFunction) {
                use self::PinNumber::*;
                use stm32f7::stm32f7x6::$gpio::afrh::AFRH15W;
                use stm32f7::stm32f7x6::$gpio::afrl::AFRL7W;

                let variant = || match alternate_fn {
                    AlternateFunction::AF0 => (AFRL7W::AF0, AFRH15W::AF0),
                    AlternateFunction::AF1 => (AFRL7W::AF1, AFRH15W::AF1),
                    AlternateFunction::AF2 => (AFRL7W::AF2, AFRH15W::AF2),
                    AlternateFunction::AF3 => (AFRL7W::AF3, AFRH15W::AF3),
                    AlternateFunction::AF4 => (AFRL7W::AF4, AFRH15W::AF4),
                    AlternateFunction::AF5 => (AFRL7W::AF5, AFRH15W::AF5),
                    AlternateFunction::AF6 => (AFRL7W::AF6, AFRH15W::AF6),
                    AlternateFunction::AF7 => (AFRL7W::AF7, AFRH15W::AF7),
                    AlternateFunction::AF8 => (AFRL7W::AF8, AFRH15W::AF8),
                    AlternateFunction::AF9 => (AFRL7W::AF9, AFRH15W::AF9),
                    AlternateFunction::AF10 => (AFRL7W::AF10, AFRH15W::AF10),
                    AlternateFunction::AF11 => (AFRL7W::AF11, AFRH15W::AF11),
                    AlternateFunction::AF12 => (AFRL7W::AF12, AFRH15W::AF12),
                    AlternateFunction::AF13 => (AFRL7W::AF13, AFRH15W::AF13),
                    AlternateFunction::AF14 => (AFRL7W::AF14, AFRH15W::AF14),
                    AlternateFunction::AF15 => (AFRL7W::AF15, AFRH15W::AF15),
                };

                self.afrh.modify(|_, wh| {
                    self.afrl.modify(|_, wl| {
                        for pin in pins {
                            match pin {
                                Pin0 => {
                                    wl.afrl0().variant(variant().0);
                                }
                                Pin1 => {
                                    wl.afrl1().variant(variant().0);
                                }
                                Pin2 => {
                                    wl.afrl2().variant(variant().0);
                                }
                                Pin3 => {
                                    wl.afrl3().variant(variant().0);
                                }
                                Pin4 => {
                                    wl.afrl4().variant(variant().0);
                                }
                                Pin5 => {
                                    wl.afrl5().variant(variant().0);
                                }
                                Pin6 => {
                                    wl.afrl6().variant(variant().0);
                                }
                                Pin7 => {
                                    wl.afrl7().variant(variant().0);
                                }
                                Pin8 => {
                                    wh.afrh8().variant(variant().1);
                                }
                                Pin9 => {
                                    wh.afrh9().variant(variant().1);
                                }
                                Pin10 => {
                                    wh.afrh10().variant(variant().1);
                                }
                                Pin11 => {
                                    wh.afrh11().variant(variant().1);
                                }
                                Pin12 => {
                                    wh.afrh12().variant(variant().1);
                                }
                                Pin13 => {
                                    wh.afrh13().variant(variant().1);
                                }
                                Pin14 => {
                                    wh.afrh14().variant(variant().1);
                                }
                                Pin15 => {
                                    wh.afrh15().variant(variant().1);
                                }
                            };
                        }
                        wl
                    });
                    wh
                })
            }
        }
    };
}

impl_register_block_trait!(RegisterBlockA, gpioa);
impl_register_block_trait!(RegisterBlockB, gpiob);
impl_register_block_trait!(RegisterBlockD, gpiod);
