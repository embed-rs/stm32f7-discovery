//! Safe abstractions for an I2C bus.

use core::iter::TrustedLen;
use core::marker::PhantomData;
use core::ops::Deref;
use embedded_hal;
use stm32f7::stm32f7x6::{self as device, i2c1, RCC};

/// This trait marks all valid I2C types. Used to provide generic interfaces.
///
/// TODO: replace by trait alias when they're fully implemented
pub trait I2cTrait: Deref<Target = i2c1::RegisterBlock> {}

impl I2cTrait for device::I2C1 {}
impl I2cTrait for device::I2C2 {}
impl I2cTrait for device::I2C3 {}

/// Represents an I2C (Inter-Integrated Circuit) bus.
pub struct I2C<I: I2cTrait>(I);

/// Errors that can happen while accessing the I2C bus.
#[derive(Debug)]
pub enum Error {
    /// A NACK flag (negative acknowledgement) was detected.
    Nack,
}

/// An I2C address.
///
/// Currently only 7 bit addresses are supported.
#[derive(Debug, Clone, Copy)]
pub struct Address(u16);

impl Address {
    /// Create a 7 bit I2C address.
    pub const fn bits_7(addr: u8) -> Address {
        Address((addr as u16) << 1)
    }
}

fn icr_clear_all(w: &mut i2c1::icr::W) -> &mut i2c1::icr::W {
    w.alertcf().set_bit(); // alert clear flag
    w.timoutcf().set_bit(); // timeout detection clear flag
    w.peccf().set_bit(); // PEC error clear flag
    w.ovrcf().set_bit(); // overrun/underrun clear flag
    w.arlocf().set_bit(); // arbitration loss clear flag
    w.berrcf().set_bit(); // bus error clear flag
    w.stopcf().set_bit(); // stop detection clear flag
    w.nackcf().set_bit(); // not acknowledge clear flag
    w.addrcf().set_bit(); // address matched clear flag
    w
}

/// An active connection to a device on the I2C bus.
///
/// Allows reading and writing the registers of the device.
pub struct I2cConnection<'a, I: I2cTrait, T: RegisterType> {
    i2c: &'a mut I2C<I>,
    device_address: Address,
    register_type: PhantomData<T>,
}

/// Valid register types of I2C devices.
///
/// This trait is implemented for the `u8` and `u16` types.
pub trait RegisterType: Sized {
    /// Convert the register type into a byte slice and pass it to the specified closure.
    fn write<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&[u8]) -> Result<(), Error>;

    /// Call the specified closure with a mutable reference to a byte slice and then convert it
    /// to the register type.
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

impl<'a, I: I2cTrait, T: RegisterType> I2cConnection<'a, I, T> {
    fn start(&mut self, read: bool, bytes: u8) {
        self.i2c.0.cr2.write(|w| {
            w.sadd().bits(self.device_address.0); // slave_address
            w.start().set_bit(); // start_generation
            w.rd_wrn().bit(read); // read_transfer
            w.nbytes().bits(bytes); // number_of_bytes
            w.autoend().clear_bit(); // automatic_end_mode
            w
        })
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
            self.i2c.0.txdr.modify(|_, w| w.txdata().bits(b)); // transmit_data
        }

        self.i2c.wait_for_transfer_complete()?;

        self.clear_status_flags();

        // reset cr2
        self.i2c.0.cr2.write(|w| w);

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
            *b = self.i2c.0.rxdr.read().rxdata().bits(); // receive_data
        }

        self.i2c.wait_for_transfer_complete()?;

        self.clear_status_flags();

        // reset cr2
        self.i2c.0.cr2.write(|w| w);

        Ok(())
    }

    fn pre(&mut self) {
        self.clear_status_flags();
        // flush transmit data register
        self.i2c.0.isr.modify(|_, w| w.txe().set_bit()); // flush_txdr
    }

    fn clear_status_flags(&mut self) {
        self.i2c.0.icr.write(|w| icr_clear_all(w));
    }

    /// Read the current value from the specified register.
    pub fn read(&mut self, register_address: T) -> Result<T, Error> {
        self.pre();

        register_address.write(|addr_bytes| self.write_bytes(addr_bytes.iter().cloned()))?;

        T::read(|val_bytes| self.read_bytes_raw(val_bytes.iter_mut()))
    }

    /// Read bytes from the specified register into the specified buffer.
    pub fn read_bytes(&mut self, register_address: T, bytes: &mut [u8]) -> Result<(), Error> {
        self.pre();

        register_address.write(|addr_bytes| self.write_bytes(addr_bytes.iter().cloned()))?;

        self.read_bytes_raw(bytes.iter_mut())
    }

    /// Write the specified bytes into to specified register.
    pub fn write(&mut self, register_address: T, value: T) -> Result<(), Error> {
        self.pre();
        register_address.write(|addr_bytes| {
            value.write(|val_bytes| {
                self.write_bytes(addr_bytes.iter().cloned().chain(val_bytes.iter().cloned()))
            })
        })
    }
}

impl<I: I2cTrait> I2C<I> {
    /// Connects to the specified device and run the closure `f` with the connection as argument.
    ///
    /// This function takes an exclusive reference to the `I2C` type because it blocks the I2C
    /// bus. The connection is active until the closure `f` returns.
    pub fn connect<T, F>(&mut self, device_address: Address, f: F) -> Result<(), Error>
    where
        T: RegisterType,
        F: FnOnce(I2cConnection<I, T>) -> Result<(), Error>,
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

    /// Stop the active connection by sending a stop symbol.
    pub fn stop(&mut self) -> Result<(), Error> {
        self.0.cr2.modify(|_, w| w.stop().set_bit());

        // reset cr2
        self.0.cr2.write(|w| w);

        self.wait_for_stop()
    }

    /// Update a device register.
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
            if isr.nackf().bit_is_set() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.txis().bit_is_set() {
                return Ok(());
            }
        }
    }

    /// Wait for "receive data register not empty" flag
    fn wait_for_rxne(&self) -> Result<(), Error> {
        loop {
            let isr = self.0.isr.read();
            if isr.nackf().bit_is_set() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.rxne().bit_is_set() {
                return Ok(());
            }
        }
    }

    /// Wait for “transfer complete” flag
    fn wait_for_transfer_complete(&self) -> Result<(), Error> {
        loop {
            let isr = self.0.isr.read();
            if isr.nackf().bit_is_set() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.tc().bit_is_set() {
                // transfer_complete
                return Ok(());
            }
        }
    }

    /// Wait for automatically generated stop flag
    fn wait_for_stop(&self) -> Result<(), Error> {
        loop {
            let isr = self.0.isr.read();
            if isr.nackf().bit_is_set() {
                // nack_received
                return Err(Error::Nack);
            }
            if isr.stopf().bit_is_set() {
                // stop_detected
                return Ok(());
            }
        }
    }

    /// Provokes a NACK and checks if the response is as expected. Panics otherwise.
    pub fn test_1(&mut self) {
        let i2c = &mut self.0;

        i2c.cr2.modify(|_, w| {
            w.sadd().bits(Address::bits_7(0b1010101).0); // slave_address
            w.start().set_bit(); // start_generation
            w.nbytes().bits(0); // number_of_bytes
            w.autoend().set_bit(); // automatic_end_mode
            w
        });

        loop {
            let isr = i2c.isr.read();
            if isr.nackf().bit_is_set() {
                // nack_received
                break;
            }
            assert!(isr.stopf().bit_is_clear()); // stop_detected
        }

        // clear status flags
        i2c.icr.write(|w| icr_clear_all(w));
    }

    /// Try to access all I2C addresses. Panics on test failure.
    #[allow(dead_code)]
    pub fn test_2(&mut self) {
        let i2c = &mut self.0;

        let mut addr = 0;
        loop {
            i2c.cr2.modify(|_, w| {
                w.sadd().bits(Address::bits_7(addr).0); // slave_address
                w.start().set_bit(); // start_generation
                w.nbytes().bits(0); // number_of_bytes
                w.autoend().set_bit(); // automatic_end_mode
                w
            });

            let mut isr = i2c.isr.read();
            loop {
                if isr.nackf().bit_is_set() || isr.stopf().bit_is_set() {
                    // nack_received or stop_detected
                    break;
                }
                isr = i2c.isr.read();
            }

            if !isr.nackf().bit_is_set() {
                let _x = addr;
            } else {
                while i2c.isr.read().busy().bit_is_set() {}
                // clear status flags
                i2c.icr.write(|w| icr_clear_all(w));
            }

            addr += 1;
            if addr >= 0x80 {
                return;
            }
        }
    }
}

impl<I: I2cTrait> embedded_hal::blocking::i2c::Read for I2C<I> {
    type Error = Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.connect(
            Address::bits_7(address),
            |mut connection: I2cConnection<I, u8>| connection.read_bytes_raw(buffer.iter_mut()),
        )
    }
}

impl<I: I2cTrait> embedded_hal::blocking::i2c::Write for I2C<I> {
    type Error = Error;

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.connect(
            Address::bits_7(address),
            |mut connection: I2cConnection<I, u8>| connection.write_bytes(bytes.iter().map(|b| *b)),
        )
    }
}

impl<I: I2cTrait> embedded_hal::blocking::i2c::WriteRead for I2C<I> {
    type Error = Error;

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.connect(
            Address::bits_7(address),
            |mut connection: I2cConnection<I, u8>| {
                connection.write_bytes(bytes.iter().map(|b| *b))?;
                connection.read_bytes_raw(buffer.iter_mut())
            },
        )
    }
}

/// Initialize the I2C bus and return an `I2C` type.
pub fn init<I: I2cTrait>(i2c: I, rcc: &mut RCC) -> I2C<I> {
    // enable clocks
    rcc.apb1enr.modify(|_, w| w.i2c3en().enabled());

    // disable I2C peripheral
    i2c.cr1.modify(|_, w| w.pe().clear_bit()); // peripheral_enable register

    // configure timing register TODO: check/understand values
    i2c.timingr.modify(|_, w| {
        w.presc().bits(0x4); // timing_prescaler
        w.scldel().bits(0x9); // data_setup_time
        w.sdadel().bits(0x1); // data_hold_time
        w.sclh().bits(0x27); // scl_high_period
        w.scll().bits(0x32); // scl_low_period
        w
    });

    // configure oar1
    i2c.oar1.modify(|_, w| w.oa1en().clear_bit()); // own_address_1_enable register
    i2c.oar1.modify(|_, w| {
        w.oa1().bits(0x00); // own_address_1
        w.oa1mode().clear_bit(); // 10 bit mode
        w.oa1en().clear_bit(); // TODO
        w
    });

    // configure cr2
    i2c.cr2.modify(|_, w| {
        w.add10().clear_bit(); // 10_bit_addressing mode
        w.autoend().clear_bit(); // automatic_end_mode
        w
    });

    // configure oar2
    i2c.oar2.modify(|_, w| {
        w.oa2en().clear_bit() // own_address_2_enable
    });

    // configure cr1
    i2c.cr1.modify(|_, w| {
        w.gcen().clear_bit(); // general_call
        w.nostretch().clear_bit(); // clock_stretching_disable
        w.pe().set_bit(); // peripheral_enable
        w
    });
    // wait that init can finish
    crate::system_clock::wait_ms(50);

    I2C(i2c)
}
