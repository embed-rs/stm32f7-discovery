#![allow(dead_code)]

use svd_board::gpiod::{self, Moder, Ospeedr, Otyper, Pupdr, Afrl, Afrh, Idr, Odr, Bsrr};
use volatile::{ReadOnly, WriteOnly, ReadWrite};
use core::marker::PhantomData;
use super::port::PortNumber;

pub struct GpioPin<Port: PortNumber, Pin: PinNumber> {
    _phantom_1: PhantomData<Port>,
    _phantom_2: PhantomData<Pin>,
}

pub fn new_pin<Port, Pin>() -> GpioPin<Port, Pin>
    where Port: PortNumber,
          Pin: PinNumber
{
    GpioPin {
        _phantom_1: PhantomData,
        _phantom_2: PhantomData,
    }
}

pub struct GpioRead<Pin: PinNumber> {
    idr: &'static ReadOnly<gpiod::Idr>,
    _phantom: PhantomData<Pin>,
}

pub fn new_read<Pin: PinNumber>(idr: &'static ReadOnly<gpiod::Idr>) -> GpioRead<Pin> {
    GpioRead {
        idr: idr,
        _phantom: PhantomData,
    }
}

impl<Pin: PinNumber> GpioRead<Pin> {
    pub fn read(&self) -> bool {
        Pin::input(&self.idr.read())
    }
}

pub struct GpioWrite<Pin: PinNumber> {
    odr: &'static ReadWrite<gpiod::Odr>,
    bsrr: BsrrRef<Pin>,
}

pub fn new_write<Pin: PinNumber>(odr: &'static ReadWrite<gpiod::Odr>,
                                 bsrr: *mut WriteOnly<gpiod::Bsrr>)
                                 -> GpioWrite<Pin> {
    GpioWrite {
        odr: odr,
        bsrr: BsrrRef {
            bsrr: bsrr,
            _phantom: PhantomData,
        },
    }
}

impl<Pin: PinNumber> GpioWrite<Pin> {
    pub fn current(&self) -> bool {
        Pin::current_output(&self.odr.read())
    }
    pub fn set(&mut self, high: bool) {
        self.bsrr.set(high);
    }
}

#[derive(Debug, Clone)]
struct BsrrRef<Pin: PinNumber> {
    bsrr: *mut WriteOnly<gpiod::Bsrr>,
    _phantom: PhantomData<Pin>,
}

impl<Pin: PinNumber> BsrrRef<Pin> {
    fn set(&self, value: bool) {
        let mut new_value = Default::default();
        Pin::set_output(&mut new_value, value);
        let bsrr = unsafe { &mut *self.bsrr };
        bsrr.write(new_value);
    }
}

pub trait PinNumber {
    fn set_mode(moder: &mut Moder, value: u8);
    fn set_speed(ospeedr: &mut Ospeedr, value: u8);
    fn set_type(otyper: &mut Otyper, value: bool);
    fn set_pupd(pupdr: &mut Pupdr, value: u8);
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8);

    fn input(idr: &Idr) -> bool;
    fn current_output(odr: &Odr) -> bool;
    fn set_output(bsrr: &mut Bsrr, value: bool);
}

pub enum Pin0 {}
pub enum Pin1 {}
pub enum Pin2 {}
pub enum Pin3 {}
pub enum Pin4 {}
pub enum Pin5 {}
pub enum Pin6 {}
pub enum Pin7 {}
pub enum Pin8 {}
pub enum Pin9 {}
pub enum Pin10 {}
pub enum Pin11 {}
pub enum Pin12 {}
pub enum Pin13 {}
pub enum Pin14 {}
pub enum Pin15 {}

impl PinNumber for Pin0 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder0(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr0(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot0(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr0(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl0(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr0()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr0()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs0(true);
        } else {
            bsrr.set_br0(true);
        }
    }
}

impl PinNumber for Pin1 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder1(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr1(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot1(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr1(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl1(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr1()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr1()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs1(true);
        } else {
            bsrr.set_br1(true);
        }
    }
}

impl PinNumber for Pin2 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder2(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr2(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot2(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr2(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl2(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr2()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr2()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs2(true);
        } else {
            bsrr.set_br2(true);
        }
    }
}

impl PinNumber for Pin3 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder3(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr3(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot3(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr3(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl3(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr3()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr3()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs3(true);
        } else {
            bsrr.set_br3(true);
        }
    }
}

impl PinNumber for Pin4 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder4(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr4(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot4(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr4(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl4(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr4()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr4()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs4(true);
        } else {
            bsrr.set_br4(true);
        }
    }
}

impl PinNumber for Pin5 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder5(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr5(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot5(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr5(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl5(value));
    }


    fn input(idr: &Idr) -> bool {
        idr.idr5()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr5()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs5(true);
        } else {
            bsrr.set_br5(true);
        }
    }
}

impl PinNumber for Pin6 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder6(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr6(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot6(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr6(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl6(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr6()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr6()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs6(true);
        } else {
            bsrr.set_br6(true);
        }
    }
}

impl PinNumber for Pin7 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder7(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr7(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot7(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr7(value);
    }
    fn set_alternate_fn(afrl: &mut ReadWrite<Afrl>, _afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrl.update(|r| r.set_afrl7(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr7()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr7()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs7(true);
        } else {
            bsrr.set_br7(true);
        }
    }
}

impl PinNumber for Pin8 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder8(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr8(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot8(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr8(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh8(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr8()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr8()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs8(true);
        } else {
            bsrr.set_br8(true);
        }
    }
}

impl PinNumber for Pin9 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder9(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr9(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot9(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr9(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh9(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr9()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr9()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs9(true);
        } else {
            bsrr.set_br9(true);
        }
    }
}

impl PinNumber for Pin10 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder10(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr10(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot10(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr10(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh10(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr10()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr10()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs10(true);
        } else {
            bsrr.set_br10(true);
        }
    }
}

impl PinNumber for Pin11 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder11(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr11(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot11(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr11(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh11(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr11()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr11()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs11(true);
        } else {
            bsrr.set_br11(true);
        }
    }
}

impl PinNumber for Pin12 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder12(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr12(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot12(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr12(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh12(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr12()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr12()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs12(true);
        } else {
            bsrr.set_br12(true);
        }
    }
}

impl PinNumber for Pin13 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder13(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr13(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot13(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr13(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh13(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr13()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr13()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs13(true);
        } else {
            bsrr.set_br13(true);
        }
    }
}

impl PinNumber for Pin14 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder14(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr14(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot14(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr14(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh14(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr14()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr14()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs14(true);
        } else {
            bsrr.set_br14(true);
        }
    }
}

impl PinNumber for Pin15 {
    fn set_mode(moder: &mut Moder, value: u8) {
        moder.set_moder15(value);
    }
    fn set_speed(ospeedr: &mut Ospeedr, value: u8) {
        ospeedr.set_ospeedr15(value);
    }
    fn set_type(otyper: &mut Otyper, value: bool) {
        otyper.set_ot15(value);
    }
    fn set_pupd(pupdr: &mut Pupdr, value: u8) {
        pupdr.set_pupdr15(value);
    }
    fn set_alternate_fn(_afrl: &mut ReadWrite<Afrl>, afrh: &mut ReadWrite<Afrh>, value: u8) {
        afrh.update(|r| r.set_afrh15(value));
    }

    fn input(idr: &Idr) -> bool {
        idr.idr15()
    }
    fn current_output(odr: &Odr) -> bool {
        odr.odr15()
    }
    fn set_output(bsrr: &mut Bsrr, value: bool) {
        if value {
            bsrr.set_bs15(true);
        } else {
            bsrr.set_br15(true);
        }
    }
}
