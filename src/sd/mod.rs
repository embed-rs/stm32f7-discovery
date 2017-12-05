pub use self::init::init;
pub use self::init::de_init;

pub mod error;
mod init;
mod sdmmc_cmd;

use board::sdmmc::Sdmmc;
use board::rcc::Rcc;
use embedded::interfaces::gpio::{Gpio, InputPin};
use self::error::*;
use core::cmp::min;
use alloc::vec::Vec;

pub struct Sd {
    sdmmc: &'static mut Sdmmc,
    card_info: Option<CardInfo>,
    present_pin: InputPin,
}

impl Sd {
    pub fn new(sdmmc: &'static mut Sdmmc, gpio: &mut Gpio, rcc: &mut Rcc) -> Sd {
        use embedded::interfaces::gpio::Port::*;
        use embedded::interfaces::gpio::Pin::*;
        use embedded::interfaces::gpio::Resistor;

        rcc.ahb1enr.update(|r| r.set_gpiocen(true));
        // wait for enabling
        while !rcc.ahb1enr.read().gpiocen() {}

        let present_pin = gpio.to_input((PortC, Pin13), Resistor::PullUp).unwrap();

        self::init::init_hw(gpio, rcc);

        Sd {
            sdmmc: sdmmc,
            card_info: None,
            present_pin: present_pin,
        }
    }
    pub fn get_card_info(&self) -> &Option<CardInfo> {
        &self.card_info
    }
    pub fn card_present(&self) -> bool {
        !self.present_pin.get()
    }
    pub fn card_initialized(&self) -> bool {
        self.card_info.is_some()
    }
    pub fn read_blocks(&mut self, block_add: u32, number_of_blks: u16) -> Result<Vec<u32>, Error> {
        let mut data = vec![];
        for i in 0..u32::from(number_of_blks) {
            let mut block = self.read_blocks_h(block_add + i, 1, 5000)?;
            data.append(&mut block);
        }

        Ok(data)
    }
    pub fn write_blocks(&mut self, data: &[u32], block_add: u32, number_of_blks: u16) -> Result<(), Error> {
        for i in 0..u32::from(number_of_blks) {
            self.write_blocks_h(&data[min((i as usize)*128, data.len())..], block_add + i, 1, 5000)?;
        }

        Ok(())
    }

    fn read_blocks_h(
        &mut self,
        block_add: u32,
        number_of_blks: u16,
        timeout: u32) -> Result<Vec<u32>, Error> {
        if number_of_blks == 0 {
            return Ok(vec![])
        }
        if !self.card_present() {
            return Err(Error::NoSdCard)
        }
        let mut block_add = block_add;
        let card_info = self.card_info.as_ref().unwrap();

        if block_add + u32::from(number_of_blks) > card_info.log_blk_number {
            return Err( Error::RWError { t: RWErrorType::AddressOutOfRange } )
        }

        if card_info.card_type == CardType::SDv2HC {
            block_add *= card_info.log_blk_size;
        }

        // Tell the sdmmc the block length...
        sdmmc_cmd::block_length(self.sdmmc, card_info.log_blk_size)?;
        // ...and if a single or multiple block should be read
        if number_of_blks > 1 {
            sdmmc_cmd::read_multi_blk(self.sdmmc, block_add)?;
        } else {
            sdmmc_cmd::read_single_blk(self.sdmmc, block_add)?;
        }

        // Set up the Data Path State Machine (DPSM)
        let data_length = u32::from(number_of_blks) * card_info.log_blk_size;
        self.sdmmc.dlen.update(|d| d.set_datalength(data_length));
        self.sdmmc.dtimer.update(|d| d.set_datatime(0xFFFF_FFFF));
        self.sdmmc.dctrl.update(|d| {
            d.set_dblocksize(0x09); // blocksize = 2^n => blocksize = 2^9 = 512
            d.set_dtdir(true);      // direction: false -> write, true -> read
            d.set_dtmode(false);    // mode: false -> block, true -> stream
            d.set_dten(true);       // enable data transfer
        });

        let mut data = vec![];
        let timeout = ::system_clock::ticks() as u32 + timeout;
        while (::system_clock::ticks() as u32) < timeout
            && !self.sdmmc.sta.read().rxoverr()
            && !self.sdmmc.sta.read().dcrcfail()
            && !self.sdmmc.sta.read().dtimeout()
            && !self.sdmmc.sta.read().dataend()
        {
            if self.sdmmc.sta.read().rxfifohf() {
                for _ in 0..8 {
                    data.push(self.sdmmc.fifo.read().fifodata());
                }
            }
        }

        if (::system_clock::ticks() as u32) >= timeout {
            return Err(Error::Timeout);
        }

        if self.sdmmc.sta.read().dataend() && number_of_blks > 1 {
            sdmmc_cmd::stop_transfer(self.sdmmc)?;
        }

        if self.sdmmc.sta.read().dtimeout() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError { t: RWErrorType::DataTimeout });
        }
        if self.sdmmc.sta.read().dcrcfail() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError { t: RWErrorType::DataCrcFailed });
        }
        if self.sdmmc.sta.read().rxoverr() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError { t: RWErrorType::RxOverrun });
        }

        while (::system_clock::ticks() as u32) < timeout
            && self.sdmmc.sta.read().rxdavl() {
            data.push(self.sdmmc.fifo.read().fifodata());
        }

        if (::system_clock::ticks() as u32) >= timeout {
            return Err(Error::Timeout);
        }

        sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);

        Ok(data)
    }

    fn write_blocks_h(
        &mut self,
        data: &[u32],
        block_add: u32,
        number_of_blks: u16,
        timeout: u32) -> Result<(), Error> {
        if number_of_blks == 0 {
            return Ok(())
        }
        if !self.card_present() {
            return Err(Error::NoSdCard)
        }
        let mut block_add = block_add;
        let card_info = self.card_info.as_ref().unwrap();

        if block_add + u32::from(number_of_blks) > card_info.log_blk_number {
            return Err( Error::RWError { t: RWErrorType::AddressOutOfRange } )
        }

        if card_info.card_type == CardType::SDv2HC {
            block_add *= card_info.log_blk_size;
        }

        // Tell the sdmmc the block length...
        sdmmc_cmd::block_length(self.sdmmc, card_info.log_blk_size)?;
        // ...and if a single or multiple block should be written
        if number_of_blks > 1 {
            sdmmc_cmd::write_multi_blk(self.sdmmc, block_add)?;
        } else {
            sdmmc_cmd::write_single_blk(self.sdmmc, block_add)?;
        }

        // Set up the Data Path State Machine (DPSM)
        let data_length = u32::from(number_of_blks) * card_info.log_blk_size;
        self.sdmmc.dlen.update(|d| d.set_datalength(data_length));
        self.sdmmc.dtimer.update(|d| d.set_datatime(0xFFFF_FFFF));
        self.sdmmc.dctrl.update(|d| {
            d.set_dblocksize(0x09); // blocksize = 2^n => blocksize = 2^9 = 512
            d.set_dtdir(false);     // direction: false -> write, true -> read
            d.set_dtmode(false);    // mode: false -> block, true -> stream
            d.set_dten(true);       // enable data transfer
        });

        let mut data_counter = 0;
        let timeout = ::system_clock::ticks() as u32 + timeout;
        while (::system_clock::ticks() as u32) < timeout
            && !self.sdmmc.sta.read().txunderr()
            && !self.sdmmc.sta.read().dcrcfail()
            && !self.sdmmc.sta.read().dtimeout()
            && !self.sdmmc.sta.read().dataend()
        {
            if self.sdmmc.sta.read().txfifohe() {
                // if there is no more data to write, but the sdmmc controller has not reached
                // dataend yet, write 0s to the fifo
                let mut pad_data: &[u32] = &[0; 8][..];
                if data_counter < data.len() {
                    pad_data = &data[data_counter..min(data_counter + 8, data.len())];
                    data_counter += 8;
                }
                for d in pad_data {
                    self.sdmmc.fifo.update(|f| f.set_fifodata(*d));
                }
            }
        }

        if (::system_clock::ticks() as u32) >= timeout {
            return Err(Error::Timeout);
        }

        if self.sdmmc.sta.read().dataend() && number_of_blks > 1 {
            sdmmc_cmd::stop_transfer(self.sdmmc)?;
        }

        let wait = ::system_clock::ticks() + 100;
        while ::system_clock::ticks() < wait {}

        if self.sdmmc.sta.read().dtimeout() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError { t: RWErrorType::DataTimeout });
        }
        if self.sdmmc.sta.read().dcrcfail() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError { t: RWErrorType::DataCrcFailed });
        }
        if self.sdmmc.sta.read().txunderr() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError { t: RWErrorType::TxUnderrun });
        }

        sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    SDv1,
    SDv2SC,
    SDv2HC,
}

#[derive(Debug)]
pub struct CardInfo {
    card_type: CardType,
    rca: u16,
    blk_number: u32,
    blk_size: u32,
    log_blk_number: u32,
    log_blk_size: u32,
}

impl Default for CardInfo {
    fn default() -> CardInfo {
        CardInfo {
            card_type: CardType::SDv2HC,
            rca: 0,
            blk_number: 0,
            blk_size: 0,
            log_blk_number: 0,
            log_blk_size: 0,
        }
    }
}
