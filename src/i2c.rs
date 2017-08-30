#![allow(dead_code)]

use board::rcc::Rcc;
use board::i2c;
use embedded::interfaces::gpio::Gpio;
use core::marker::PhantomData;
use core::iter::TrustedLen;

pub struct I2C(&'static mut i2c::I2c);

#[derive(Debug)]
pub enum Error {
    Nack,
}

#[derive(Debug, Clone, Copy)]
pub struct Address(u16);

impl Address {
    pub const fn bits_7(addr: u8) -> Address {
        Address((addr as u16) << 1)
    }
}

pub fn init_pins_and_clocks(rcc: &mut Rcc, gpio: &mut Gpio) {
    use embedded::interfaces::gpio::{AlternateFunction, OutputSpeed, OutputType, Resistor};
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;

    // enable clocks
    rcc.apb1enr.update(|r| {
        r.set_i2c1en(true);
        r.set_i2c2en(true);
        r.set_i2c3en(true);
        r.set_i2c4en(true);
    });

    let i2c1_scl = (PortB, Pin8);
    let i2c1_sda = (PortB, Pin9);
    let i2c2_scl = (PortB, Pin10);
    let i2c2_sda = (PortB, Pin11);
    let i2c3_scl = (PortH, Pin7);
    let i2c3_sda = (PortH, Pin8);
    let i2c4_scl = (PortH, Pin11);
    let i2c4_sda = (PortD, Pin13);

    let pins = [
        i2c1_scl,
        i2c1_sda,
        i2c2_scl,
        i2c2_sda,
        i2c3_scl,
        i2c3_sda,
        i2c4_scl,
        i2c4_sda,
    ];
    gpio.to_alternate_function_all(
        &pins,
        AlternateFunction::AF4,
        OutputType::OpenDrain,
        OutputSpeed::Medium,
        Resistor::PullUp,
    ).unwrap();
}

pub fn init(i2c: &'static mut i2c::I2c) -> I2C {
    // disable I2C peripheral
    i2c.cr1.update(|r| r.set_pe(false)); // peripheral_enable

    // configure timing register TODO: check/understand values
    i2c.timingr.update(|r| {
        r.set_presc(0x4); // timing_prescaler
        r.set_scldel(0x9); // data_setup_time
        r.set_sdadel(0x1); // data_hold_time
        r.set_sclh(0x27); // scl_high_period
        r.set_scll(0x32); // scl_low_period
    });

    // configure oar1
    i2c.oar1.update(|r| r.set_oa1en(false)); // own_address_1_enable
    i2c.oar1.update(|r| {
        r.set_oa1(0x00); // own_address_1
        r.set_oa1mode(false); // 10 bit mode
        r.set_oa1en(false); // TODO
    });

    // configure cr2
    i2c.cr2.update(|r| {
        r.set_add10(false); // 10_bit_addressing mode
        r.set_autoend(false); // automatic_end_mode
    });

    // configure oar2
    i2c.oar2.update(|r| {
        r.set_oa2en(false); // own_address_2_enable
    });

    // configure cr1
    i2c.cr1.update(|r| {
        r.set_gcen(false); // general_call
        r.set_nostretch(false); // clock_stretching_disable
        r.set_pe(true); // peripheral_enable
    });
    // wait that init can finish
    ::system_clock::wait(50);
    I2C(i2c)
}

fn icr_clear_all() -> i2c::Icr {
    let mut clear_all = i2c::Icr::default();
    clear_all.set_alertcf(true); // alert clear flag
    clear_all.set_timoutcf(true); // timeout detection clear flag
    clear_all.set_peccf(true); // PEC error clear flag
    clear_all.set_ovrcf(true); // overrun/underrun clear flag
    clear_all.set_arlocf(true); // arbitration loss clear flag
    clear_all.set_berrcf(true); // bus error clear flag
    clear_all.set_stopcf(true); // stop detection clear flag
    clear_all.set_nackcf(true); // not acknowledge clear flag
    clear_all.set_addrcf(true); // address matched clear flag
    clear_all
}

pub struct I2cConnection<'a, T: RegisterType> {
    i2c: &'a mut I2C,
    device_address: Address,
    register_type: PhantomData<T>,
}

pub trait RegisterType: Sized {
    fn write<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&[u8]) -> Result<(), Error>;
    fn read<F>(f: F) -> Result<Self, Error>
    where
        F: FnOnce(&mut [u8]) -> Result<(), Error>;
}

impl RegisterType for u8 {
    fn write<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&[u8]) -> Result<(), Error>,
    {
        f(&[*self])
    }

    fn read<F>(f: F) -> Result<Self, Error>
    where
        F: FnOnce(&mut [u8]) -> Result<(), Error>,
    {
        let mut buf = [0];
        f(&mut buf)?;
        Ok(buf[0])
    }
}

impl RegisterType for u16 {
    fn write<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&[u8]) -> Result<(), Error>,
    {
        f(&[(*self >> 8) as u8, *self as u8])
    }

    fn read<F>(f: F) -> Result<Self, Error>
    where
        F: FnOnce(&mut [u8]) -> Result<(), Error>,
    {
        let mut buf = [0, 0];
        f(&mut buf)?;
        Ok((buf[0] as u16) << 8 | buf[1] as u16)
    }
}

impl<'a, T: RegisterType> I2cConnection<'a, T> {
    fn start(&mut self, read: bool, bytes: u8) {
        let mut cr2 = i2c::Cr2::default();
        cr2.set_sadd(self.device_address.0); // slave_address
        cr2.set_start(true); // start_generation
        cr2.set_rd_wrn(read); // read_transfer
        cr2.set_nbytes(bytes); // number_of_bytes
        cr2.set_autoend(false); // automatic_end_mode
        self.i2c.0.cr2.write(cr2);
    }

    fn write_bytes<ITER>(&mut self, bytes: ITER) -> Result<(), Error>
    where
        ITER: Iterator<Item = u8> + TrustedLen,
    {
        assert!(bytes.size_hint().1.is_some());
        assert_eq!(
            bytes.size_hint().0 as u8 as usize,
            bytes.size_hint().0,
            "transfers > 255 bytes are not implemented yet"
        );
        self.start(false, bytes.size_hint().0 as u8);

        for b in bytes {
            self.i2c.wait_for_txis()?;
            self.i2c.0.txdr.update(|r| r.set_txdata(b)); // transmit_data
        }

        self.i2c.wait_for_transfer_complete()?;

        self.clear_status_flags();

        // reset cr2
        self.i2c.0.cr2.write(Default::default());

        Ok(())
    }

    fn read_bytes_raw<'b, ITER>(&mut self, buffer: ITER) -> Result<(), Error>
    where
        ITER: Iterator<Item = &'b mut u8> + TrustedLen,
    {
        assert!(buffer.size_hint().1.is_some());
        assert_eq!(
            buffer.size_hint().0 as u8 as usize,
            buffer.size_hint().0,
            "transfers > 255 bytes are not implemented yet"
        );
        self.start(true, buffer.size_hint().0 as u8);

        // read data from receive data register
        for b in buffer {
            self.i2c.wait_for_rxne()?;
            *b = self.i2c.0.rxdr.read().rxdata(); // receive_data
        }

        self.i2c.wait_for_transfer_complete()?;

        self.clear_status_flags();

        // reset cr2
        self.i2c.0.cr2.write(Default::default());

        Ok(())
    }

    fn pre(&mut self) {
        self.clear_status_flags();
        // flush transmit data register
        self.i2c.0.isr.update(|r| r.set_txe(true)); // flush_txdr
    }

    fn clear_status_flags(&mut self) {
        let clear_all = icr_clear_all();
        self.i2c.0.icr.write(clear_all);
    }

    pub fn read(&mut self, register_address: T) -> Result<T, Error> {
        self.pre();

        register_address.write(|addr_bytes| self.write_bytes(addr_bytes.iter().cloned()))?;

        T::read(|val_bytes| self.read_bytes_raw(val_bytes.iter_mut()))
    }

    pub fn read_bytes(&mut self, register_address: T, bytes: &mut [u8]) -> Result<(), Error> {
        self.pre();

        register_address.write(|addr_bytes| self.write_bytes(addr_bytes.iter().cloned()))?;

        self.read_bytes_raw(bytes.iter_mut())
    }

    pub fn write(&mut self, register_address: T, value: T) -> Result<(), Error> {
        self.pre();
        register_address.write(|addr_bytes| {
            value.write(|val_bytes| {
                self.write_bytes(addr_bytes.iter().cloned().chain(val_bytes.iter().cloned()))
            })
        })
    }
}

impl I2C {
    pub fn connect<T, F>(&mut self, device_address: Address, f: F) -> Result<(), Error>
    where
        T: RegisterType,
        F: FnOnce(I2cConnection<T>) -> Result<(), Error>,
    {
        {
            let conn = I2cConnection {
                i2c: self,
                device_address: device_address,
                register_type: PhantomData,
            };
            f(conn)?;
        }
        self.stop()
    }


    pub fn stop(&mut self) -> Result<(), Error> {
        self.0.cr2.update(|r| r.set_stop(true));

        // reset cr2
        self.0.cr2.write(Default::default());

        self.wait_for_stop()
    }

    pub fn update<F>(
        &mut self,
        device_address: Address,
        register_address: u16,
        f: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(&mut u16),
    {
        self.connect(device_address, |mut conn| {
            let mut value = conn.read(register_address)?;
            f(&mut value);
            conn.write(register_address, value)
        })
    }

    /// Wait for “transmit interrupt status” flag
    fn wait_for_txis(&self) -> Result<(), Error> {
        loop {
            let isr = self.0.isr.read();
            if isr.nackf() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.txis() {
                return Ok(());
            }
        }
    }

    /// Wait for "receive data register not empty" flag
    fn wait_for_rxne(&self) -> Result<(), Error> {
        loop {
            let isr = self.0.isr.read();
            if isr.nackf() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.rxne() {
                return Ok(());
            }
        }
    }

    /// Wait for “transfer complete” flag
    fn wait_for_transfer_complete(&self) -> Result<(), Error> {
        loop {
            let isr = self.0.isr.read();
            if isr.nackf() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.tc() {
                // transfer_complete
                return Ok(());
            }
        }
    }

    /// Wait for automatically generated stop flag
    fn wait_for_stop(&self) -> Result<(), Error> {
        loop {
            let isr = self.0.isr.read();
            if isr.nackf() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.stopf() {
                // stop_detected
                return Ok(());
            }
        }
    }

    // provokes a NACK
    pub fn test_1(&mut self) {
        let i2c = &mut self.0;

        i2c.cr2.update(|r| {
            r.set_sadd(Address::bits_7(0b1010101).0); // slave_address
            r.set_start(true); // start_generation
            r.set_nbytes(0); // number_of_bytes
            r.set_autoend(true); // automatic_end_mode
        });

        loop {
            let isr = i2c.isr.read();
            if isr.nackf() {
                // nack_received
                break;
            }
            assert!(!isr.stopf()); // stop_detected
        }

        // clear status flags
        i2c.icr.write(icr_clear_all());
    }

    // try all addresses
    #[allow(dead_code)]
    pub fn test_2(&mut self) {
        let i2c = &mut self.0;

        let mut addr = 0;
        loop {
            i2c.cr2.update(|r| {
                r.set_sadd(Address::bits_7(addr).0); // slave_address
                r.set_start(true); // start_generation
                r.set_nbytes(0); // number_of_bytes
                r.set_autoend(true); // automatic_end_mode
            });

            let mut isr = i2c.isr.read();
            loop {
                if isr.nackf() || isr.stopf() {
                    // nack_received or stop_detected
                    break;
                }
                isr = i2c.isr.read();
            }

            if !isr.nackf() {
                let _x = addr;
            } else {
                while i2c.isr.read().busy() {}
                // clear status flags
                i2c.icr.write(icr_clear_all());
            }

            addr += 1;
            if addr >= 0x80 {
                return;
            }
        }
    }
}

fn panic() {
    panic!();
}
