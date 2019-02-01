//! Provides functions to detect and initialize SD cards.
//!
//! **This module is currently untested!**

#![allow(missing_docs)]

pub use self::init::{de_init, init};

pub mod error;
mod init;
mod sdmmc_cmd;

use self::error::*;
use crate::gpio::InputPin;
use alloc::vec::Vec;
use core::cmp::min;
use stm32f7::stm32f7x6::{RCC, SDMMC1};

/// SD handle.
pub struct Sd<'a, PresentPin: InputPin + 'a> {
    sdmmc: &'a mut SDMMC1,
    card_info: Option<CardInfo>,
    present_pin: &'a PresentPin,
}

impl<'a, PresentPin: InputPin> Sd<'a, PresentPin> {
    /// Creates a new SD handle. It initializes the hardware, but not the card. To initialize the
    /// card a seperate call to `sd::init()` is necessary.
    /// This function returns a SD handle whether or not a SD Card is inserted.
    ///
    /// # Examples
    /// ```rust
    /// fn main(hw: board::Hardware) -> ! {
    ///     // Setup board...
    ///
    ///     // Create SD handle
    ///     let mut sd = sd::Sd::new(sdmmc, &mut gpio, rcc);
    ///     // Initialize SD Card
    ///     if let Some(i_err) = sd::init(&mut sd).err() {
    ///         hprintln!("{:?}", i_err);
    ///     }
    ///
    ///     loop {}
    /// }
    /// ```
    pub fn new(sdmmc: &'a mut SDMMC1, rcc: &mut RCC, present_pin: &'a PresentPin) -> Self {
        self::init::init_hw(rcc);

        Sd {
            sdmmc: sdmmc,
            card_info: None,
            present_pin: present_pin,
        }
    }

    /// Returns `None` if the card is not initialized or `Some(CardInfo)` if the card is
    /// initialized.
    pub fn get_card_info(&self) -> &Option<CardInfo> {
        &self.card_info
    }

    /// Returns true if a SD Card is inserted.
    pub fn card_present(&self) -> bool {
        !self.present_pin.get()
    }

    /// Returns true if the SD Card is initialized. More precisely it returns, whether the
    /// `CardInfo` is not `None`.
    pub fn card_initialized(&self) -> bool {
        self.card_info.is_some()
    }

    /// Reads `number_of_blks` blocks at address `block_add` from the SD Card. A block has a size of 512
    /// Byte.
    ///
    /// # Errors
    ///
    /// Returns an Error if a command to the SDMMC-Controller fails or a timeout occurs.
    ///
    /// # Examples
    /// ```rust
    /// fn main(hw: board::Hardware) -> ! {
    ///     // Setup board...
    ///
    ///     let mut sd = sd::Sd::new(sdmmc, &mut gpio, rcc);
    ///     sd::init(&mut sd).expect("Init failed");
    ///
    ///     match sd.read_blocks(42, 2) {
    ///         Ok(data) => {
    ///             assert!(data.len() == 256);
    ///             hprintln!("{:?}", data);
    ///         },
    ///         Err(r_err) => hprintln!("{:?}", r_err);
    ///     }
    ///
    ///     loop {}
    /// }
    /// ```
    pub fn read_blocks(&mut self, block_add: u32, number_of_blks: u16) -> Result<Vec<u32>, Error> {
        // This is a wrapper function for the read_blocks_h() function. The read_blocks_h()
        // function can only read single blocks from the card, because the multi-block mode of the
        // SDMMC-Controller doesn't work.
        let mut data = vec![];
        for i in 0..u32::from(number_of_blks) {
            let mut block = self.read_blocks_h(block_add + i, 1, 5000)?;
            data.append(&mut block);
        }

        Ok(data)
    }

    /// Writes the content of `data` to `number_of_blks` blocks at address `block_add` to the SD
    /// Card. A block has a size of 512 Byte. If the `data` slice contains more data, then the
    /// specified number of blocks can save, the rest of the data will not be written to the card.
    /// If the `data` slice is empty or contains less data, then the specified number of blocks can
    /// save, the rest of the blocks get filled with 0s.
    ///
    /// # Errors
    ///
    /// Returns an Error if a command to the SDMMC-Controller fails or a timeout occurs.
    ///
    /// # Examples
    /// ```rust
    /// fn main(hw: board::Hardware) -> ! {
    ///     // Setup board...
    ///
    ///     let mut sd = sd::Sd::new(sdmmc, &mut gpio, rcc);
    ///     sd::init(&mut sd).expect("Init failed");
    ///
    ///     let data = vec![0; 256];
    ///
    ///     if let Some(w_err) = sd.write_blocks(&data[..], 42, 2) {
    ///         hprintln!("{:?}", w_err);
    ///     }
    ///
    ///     loop {}
    /// }
    /// ```
    pub fn write_blocks(
        &mut self,
        data: &[u32],
        block_add: u32,
        number_of_blks: u16,
    ) -> Result<(), Error> {
        // This is a wrapper function for the write_blocks_h() function. The write_blocks_h()
        // function can only write single blocks to the card, because the multi-block mode of the
        // SDMMC-Controller doesn't work.
        for i in 0..u32::from(number_of_blks) {
            self.write_blocks_h(
                &data[min((i as usize) * 128, data.len())..],
                block_add + i,
                1,
                5000,
            )?;
        }

        Ok(())
    }

    // This function doesn't support multi-block read. See read_blocks().
    fn read_blocks_h(
        &mut self,
        block_add: u32,
        number_of_blks: u16,
        timeout: u32,
    ) -> Result<Vec<u32>, Error> {
        // No blocks to read -> return empty vector
        if number_of_blks == 0 {
            return Ok(vec![]);
        }
        // Check if a SD Card is inserted.
        if !self.card_present() {
            return Err(Error::NoSdCard);
        }
        let mut block_add = block_add;
        let card_info = self.card_info.as_ref().unwrap();

        // Check if the blocks to read are in bounds.
        if block_add + u32::from(number_of_blks) > card_info.log_blk_number {
            return Err(Error::RWError {
                t: RWErrorType::AddressOutOfRange,
            });
        }

        // On high capacity cards the block_add has to be in bytes and not the block number itself.
        if card_info.card_type == CardType::SDv2HC {
            block_add *= card_info.log_blk_size;
        }

        // Tell the sdmmc the block length...
        sdmmc_cmd::block_length(self.sdmmc, card_info.log_blk_size)?;
        // ...and if a single or multiple block should be read
        // TODO: multi-block read doesn't seem to work with the SDMMC-Controller
        if number_of_blks > 1 {
            sdmmc_cmd::read_multi_blk(self.sdmmc, block_add)?;
        } else {
            sdmmc_cmd::read_single_blk(self.sdmmc, block_add)?;
        }

        // Set up the Data Path State Machine (DPSM)
        let data_length = u32::from(number_of_blks) * card_info.log_blk_size;
        self.sdmmc
            .dlen
            .modify(|_, w| unsafe { w.datalength().bits(data_length) });
        self.sdmmc
            .dtimer
            .modify(|_, w| unsafe { w.datatime().bits(0xFFFF_FFFF) });
        self.sdmmc.dctrl.modify(|_, w| {
            unsafe { w.dblocksize().bits(0x09) }; // blocksize = 2^n => blocksize = 2^9 = 512
            w.dtdir().set_bit(); // direction: false -> write, true -> read
            w.dtmode().clear_bit(); // mode: false -> block, true -> stream
            w.dten().set_bit(); // enable data transfer
            w
        });

        // Read data from the SD Card, until dataend is reached or an error occurs
        let mut data = vec![];
        let timeout = crate::system_clock::ms() as u32 + timeout;
        while (crate::system_clock::ms() as u32) < timeout
            && self.sdmmc.sta.read().rxoverr().bit_is_clear()
            && self.sdmmc.sta.read().dcrcfail().bit_is_clear()
            && self.sdmmc.sta.read().dtimeout().bit_is_clear()
            && self.sdmmc.sta.read().dataend().bit_is_clear()
        {
            if self.sdmmc.sta.read().rxfifohf().bit_is_set() {
                for _ in 0..8 {
                    data.push(self.sdmmc.fifo.read().fifodata().bits());
                }
            }
        }

        if (crate::system_clock::ms() as u32) >= timeout {
            return Err(Error::Timeout);
        }

        // Needed in multi-block mode to stop the transmission.
        if self.sdmmc.sta.read().dataend().bit_is_set() && number_of_blks > 1 {
            sdmmc_cmd::stop_transfer(self.sdmmc)?;
        }

        // Check for errors
        if self.sdmmc.sta.read().dtimeout().bit_is_set() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataTimeout,
            });
        }
        if self.sdmmc.sta.read().dcrcfail().bit_is_set() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataCrcFailed,
            });
        }
        if self.sdmmc.sta.read().rxoverr().bit_is_set() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::RxOverrun,
            });
        }

        // If there is still valid data in the FIFO, empty the FIFO
        while (crate::system_clock::ms() as u32) < timeout
            && self.sdmmc.sta.read().rxdavl().bit_is_set()
        {
            data.push(self.sdmmc.fifo.read().fifodata().bits());
        }

        if (crate::system_clock::ms() as u32) >= timeout {
            return Err(Error::Timeout);
        }

        sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);

        Ok(data)
    }

    // This function doesn't support multi-block write. See write_blocks().
    fn write_blocks_h(
        &mut self,
        data: &[u32],
        block_add: u32,
        number_of_blks: u16,
        timeout: u32,
    ) -> Result<(), Error> {
        // No blocks to read -> return empty vector
        if number_of_blks == 0 {
            return Ok(());
        }
        // Check if a SD Card is inserted.
        if !self.card_present() {
            return Err(Error::NoSdCard);
        }
        let mut block_add = block_add;
        let card_info = self.card_info.as_ref().unwrap();

        // Check if the blocks to read are in bounds.
        if block_add + u32::from(number_of_blks) > card_info.log_blk_number {
            return Err(Error::RWError {
                t: RWErrorType::AddressOutOfRange,
            });
        }

        // On high capacity cards the block_add has to be in bytes and not the block number itself.
        if card_info.card_type == CardType::SDv2HC {
            block_add *= card_info.log_blk_size;
        }

        // Tell the sdmmc the block length...
        sdmmc_cmd::block_length(self.sdmmc, card_info.log_blk_size)?;
        // ...and if a single or multiple block should be written
        // TODO: multi-block write doesn't seem to work with the SDMMC-Controller
        if number_of_blks > 1 {
            sdmmc_cmd::write_multi_blk(self.sdmmc, block_add)?;
        } else {
            sdmmc_cmd::write_single_blk(self.sdmmc, block_add)?;
        }

        // Set up the Data Path State Machine (DPSM)
        let data_length = u32::from(number_of_blks) * card_info.log_blk_size;
        self.sdmmc
            .dlen
            .modify(|_, w| unsafe { w.datalength().bits(data_length) });
        self.sdmmc
            .dtimer
            .modify(|_, w| unsafe { w.datatime().bits(0xFFFF_FFFF) });
        self.sdmmc.dctrl.modify(|_, w| {
            unsafe { w.dblocksize().bits(0x09) }; // blocksize = 2^n => blocksize = 2^9 = 512
            w.dtdir().clear_bit(); // direction: false -> write, true -> read
            w.dtmode().clear_bit(); // mode: false -> block, true -> stream
            w.dten().set_bit(); // enable data transfer
            w
        });

        // Write data to the SD Card, until dataend is reached or an error occurs
        let mut data_counter = 0;
        let timeout = crate::system_clock::ms() as u32 + timeout;
        while (crate::system_clock::ms() as u32) < timeout
            && self.sdmmc.sta.read().txunderr().bit_is_clear()
            && self.sdmmc.sta.read().dcrcfail().bit_is_clear()
            && self.sdmmc.sta.read().dtimeout().bit_is_clear()
            && self.sdmmc.sta.read().dataend().bit_is_clear()
        {
            if self.sdmmc.sta.read().txfifohe().bit_is_set() {
                // If there is no more data to write, but the sdmmc controller has not reached
                // dataend yet, write 0s to the FIFO
                let mut pad_data: &[u32] = &[0; 8][..];
                if data_counter < data.len() {
                    pad_data = &data[data_counter..min(data_counter + 8, data.len())];
                    data_counter += 8;
                }
                for d in pad_data {
                    self.sdmmc
                        .fifo
                        .modify(|_, w| unsafe { w.fifodata().bits(*d) });
                }
            }
        }

        if (crate::system_clock::ms() as u32) >= timeout {
            return Err(Error::Timeout);
        }

        // Needed in multi-block mode to stop the transmission
        if self.sdmmc.sta.read().dataend().bit_is_set() && number_of_blks > 1 {
            sdmmc_cmd::stop_transfer(self.sdmmc)?;
        }

        // Wait a bit for the controller to end the write process.
        let wait = crate::system_clock::ms() + 100;
        while crate::system_clock::ms() < wait {}

        // Check for errors
        if self.sdmmc.sta.read().dtimeout().bit_is_set() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataTimeout,
            });
        }
        if self.sdmmc.sta.read().dcrcfail().bit_is_set() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataCrcFailed,
            });
        }
        if self.sdmmc.sta.read().txunderr().bit_is_set() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::TxUnderrun,
            });
        }

        sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);

        Ok(())
    }
}

/// Different SD card versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    /// SD version 1, always Standard Capacity (SD)
    SDv1,
    /// SD version 2 with SD (up to 2 GB)
    SDv2SC,
    /// SD version 2 with High Capacity (HC) (up to 32 GB) or Extended Capacity (XC) (up to 2 TB)
    SDv2HC,
}

/// Various information about the SD card.
#[derive(Debug)]
pub struct CardInfo {
    /// The type of the card.
    card_type: CardType,
    /// Relative Card Address
    rca: u16,
    /// Number of physical blocks
    blk_number: u32,
    /// Physical block size
    blk_size: u32,
    /// Number of logical blocks
    log_blk_number: u32,
    /// Logical block size
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
