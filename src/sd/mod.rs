pub use self::init::{de_init, init};

pub mod error;
mod init;
mod sdmmc_cmd;

use self::error::*;
use alloc::vec::Vec;
use board::rcc::Rcc;
use board::sdmmc::Sdmmc;
use core::cmp::min;
use embedded::interfaces::gpio::{Gpio, InputPin};

/// SD handle.
pub struct Sd {
    sdmmc: &'static mut Sdmmc,
    card_info: Option<CardInfo>,
    present_pin: InputPin,
}

impl Sd {
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
    pub fn new(sdmmc: &'static mut Sdmmc, gpio: &mut Gpio, rcc: &mut Rcc) -> Sd {
        use embedded::interfaces::gpio::Pin::*;
        use embedded::interfaces::gpio::Port::*;
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
        self.sdmmc.dlen.update(|d| d.set_datalength(data_length));
        self.sdmmc.dtimer.update(|d| d.set_datatime(0xFFFF_FFFF));
        self.sdmmc.dctrl.update(|d| {
            d.set_dblocksize(0x09); // blocksize = 2^n => blocksize = 2^9 = 512
            d.set_dtdir(true); // direction: false -> write, true -> read
            d.set_dtmode(false); // mode: false -> block, true -> stream
            d.set_dten(true); // enable data transfer
        });

        // Read data from the SD Card, until dataend is reached or an error occurs
        let mut data = vec![];
        let timeout = ::system_clock::ticks() as u32 + timeout;
        while (::system_clock::ticks() as u32) < timeout && !self.sdmmc.sta.read().rxoverr()
            && !self.sdmmc.sta.read().dcrcfail() && !self.sdmmc.sta.read().dtimeout()
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

        // Needed in multi-block mode to stop the transmission.
        if self.sdmmc.sta.read().dataend() && number_of_blks > 1 {
            sdmmc_cmd::stop_transfer(self.sdmmc)?;
        }

        // Check for errors
        if self.sdmmc.sta.read().dtimeout() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataTimeout,
            });
        }
        if self.sdmmc.sta.read().dcrcfail() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataCrcFailed,
            });
        }
        if self.sdmmc.sta.read().rxoverr() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::RxOverrun,
            });
        }

        // If there is still valid data in the FIFO, empty the FIFO
        while (::system_clock::ticks() as u32) < timeout && self.sdmmc.sta.read().rxdavl() {
            data.push(self.sdmmc.fifo.read().fifodata());
        }

        if (::system_clock::ticks() as u32) >= timeout {
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
        self.sdmmc.dlen.update(|d| d.set_datalength(data_length));
        self.sdmmc.dtimer.update(|d| d.set_datatime(0xFFFF_FFFF));
        self.sdmmc.dctrl.update(|d| {
            d.set_dblocksize(0x09); // blocksize = 2^n => blocksize = 2^9 = 512
            d.set_dtdir(false); // direction: false -> write, true -> read
            d.set_dtmode(false); // mode: false -> block, true -> stream
            d.set_dten(true); // enable data transfer
        });

        // Write data to the SD Card, until dataend is reached or an error occurs
        let mut data_counter = 0;
        let timeout = ::system_clock::ticks() as u32 + timeout;
        while (::system_clock::ticks() as u32) < timeout && !self.sdmmc.sta.read().txunderr()
            && !self.sdmmc.sta.read().dcrcfail() && !self.sdmmc.sta.read().dtimeout()
            && !self.sdmmc.sta.read().dataend()
        {
            if self.sdmmc.sta.read().txfifohe() {
                // If there is no more data to write, but the sdmmc controller has not reached
                // dataend yet, write 0s to the FIFO
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

        // Needed in multi-block mode to stop the transmission
        if self.sdmmc.sta.read().dataend() && number_of_blks > 1 {
            sdmmc_cmd::stop_transfer(self.sdmmc)?;
        }

        // Wait a bit for the controller to end the write process.
        let wait = ::system_clock::ticks() + 100;
        while ::system_clock::ticks() < wait {}

        // Check for errors
        if self.sdmmc.sta.read().dtimeout() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataTimeout,
            });
        }
        if self.sdmmc.sta.read().dcrcfail() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::DataCrcFailed,
            });
        }
        if self.sdmmc.sta.read().txunderr() {
            sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);
            return Err(Error::RWError {
                t: RWErrorType::TxUnderrun,
            });
        }

        sdmmc_cmd::clear_all_static_status_flags(self.sdmmc);

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    SDv1,   // SD version 1, always Standard Capacity (SD)
    SDv2SC, // SD version 2 with SD (up to 2 GB)
    SDv2HC, // SD version 2 with High Capacity (HC) (up to 32 GB) or Extended Capacity (XC) (up to 2 TB)
}

#[derive(Debug)]
pub struct CardInfo {
    card_type: CardType,
    rca: u16,            // Relative Card Address
    blk_number: u32,     // Number of physical blocks
    blk_size: u32,       // Physical block size
    log_blk_number: u32, // Number of logical blocks
    log_blk_size: u32,   // Logical block size
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
