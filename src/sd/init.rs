use super::error::Error;
use super::{sdmmc_cmd, CardInfo, CardType, Sd};
use crate::gpio::InputPin;
use stm32f7::stm32f7x6::{RCC, SDMMC1};

/// Initializes the SD Card, if it is inserted and not already initialized. If the card is already
/// initialized this function does nothing and returns no error.
///
/// # Errors
///
/// This function returns `NoSdCard` Error, if there is no SD Card inserted. This function also
/// fails, if a command to the SDMMC-Controller fails.
///
/// # Examples
/// Initialization, if a card is inserted on startup.
///
/// ```rust
/// fn main(hw: board::Hardware) -> ! {
///     // Setup board...
///
///     let mut sd = sd::Sd::new(sdmmc, &mut gpio, rcc);
///     sd::init(&mut sd).expect("Init failed");
///
///     loop {}
/// }
/// ```
///
/// On-the-fly (de-)initialization of the SD Card.
///
/// ```rust
/// fn main(hw: board::Hardware) -> ! {
///     // Setup board...
///
///     let mut sd = sd::Sd::new(sdmmc, &mut gpio, rcc);
///
///     loop {
///         if sd.card_present() && !sd.card_initialized() {
///             if let Some(i_err) = sd::init(&mut sd).err() {
///                 hprintln!("{:?}", i_err);
///             }
///         } else if !sd.card_present() && sd.card_initialized() {
///             sd::de_init(&mut sd);
///         }
///     }
/// }
/// ```
// TODO: Automate the (de-)initialization with interupts?
pub fn init<P: InputPin>(sd: &mut Sd<P>) -> Result<(), Error> {
    // Check for SD card
    if !sd.card_present() {
        return Err(Error::NoSdCard);
    }

    // Card already initialized
    if sd.card_initialized() {
        return Ok(());
    }

    // default clock configuration
    sd.sdmmc.clkcr.modify(|_, w| {
        w.negedge().clear_bit();
        w.bypass().clear_bit();
        w.pwrsav().clear_bit();
        w.hwfc_en().clear_bit();
        unsafe {
            w.widbus().bits(0);
            w.clkdiv().bits(0x76);
        }
        w
    });

    let mut card_info = CardInfo::default();
    card_info.card_type = power_on(sd.sdmmc)?;

    // Let the card send the CID and enter identification process
    sdmmc_cmd::send_cid(sd.sdmmc)?;

    // Get the RCA of the card
    card_info.rca = sdmmc_cmd::set_rel_add(sd.sdmmc)?;

    sdmmc_cmd::send_csd(sd.sdmmc, u32::from(card_info.rca) << 16)?;

    let csd = [
        sd.sdmmc.resp1.read().cardstatus1().bits(),
        sd.sdmmc.resp2.read().cardstatus2().bits(),
        sd.sdmmc.resp3.read().cardstatus3().bits(),
        sd.sdmmc.resp4.read().cardstatus4().bits(),
    ];

    get_card_csd(&mut card_info, csd);

    sdmmc_cmd::sel_desel(sd.sdmmc, u32::from(card_info.rca) << 16)?;

    sd.card_info = Some(card_info);

    Ok(())
}

/// Deinitializes the SD Card.
pub fn de_init<P: InputPin>(sd: &mut Sd<P>) {
    sd.card_info = None;

    sd.sdmmc
        .power
        .modify(|_, w| unsafe { w.pwrctrl().bits(0x00) });
}

/// Initializes the hardware, including the clocks used by the SDMMC-Controller.
pub fn init_hw(rcc: &mut RCC) {
    // Enable SDMMC1 clock
    rcc.apb2enr.modify(|_, w| w.sdmmc1en().enabled());

    // wait for enabling
    while !rcc.apb2enr.read().sdmmc1en().is_enabled() {}
}

fn power_on(sdmmc: &mut SDMMC1) -> Result<CardType, Error> {
    // power up the card
    sdmmc.clkcr.modify(|_, w| w.clken().clear_bit());
    sdmmc.power.modify(|_, w| unsafe { w.pwrctrl().bits(0x03) });
    sdmmc.clkcr.modify(|_, w| w.clken().set_bit());

    let mut card_type = CardType::SDv1;

    // set sd card to idle state
    sdmmc_cmd::idle(sdmmc, 5000)?;

    // get Card version and operation voltage
    let mut count = 0;
    let max_volt_trial = 0xFFFF;
    let mut valid_voltage = false;
    if sdmmc_cmd::oper_cond(sdmmc).is_ok() {
        let mut card_status = 0;
        // voltage trial for card V2
        while !valid_voltage {
            if count == max_volt_trial {
                return Err(Error::InvalidVoltrange);
            }
            count += 1;

            // Send CMD55, needed for next CMD.
            sdmmc_cmd::app(sdmmc, 0)?;

            // Send ACMD41. 0x40..0 for high capacity.
            sdmmc_cmd::app_oper(sdmmc, 0x4000_0000)?;

            card_status = sdmmc.resp1.read().cardstatus1().bits();

            valid_voltage = card_status >> 31 == 1
        }
        // determine whether high or standard capacity.
        if card_status & 0x4000_0000 != 0 {
            card_type = CardType::SDv2HC;
        } else {
            card_type = CardType::SDv2SC;
        }
    } else {
        while !valid_voltage {
            if count == max_volt_trial {
                return Err(Error::InvalidVoltrange);
            }
            count += 1;

            // Send CMD55, needed for next CMD.
            sdmmc_cmd::app(sdmmc, 0)?;

            // Send ACMD41. 0x0 for standard capacity.
            sdmmc_cmd::app_oper(sdmmc, 0x0)?;

            let card_status = sdmmc.resp1.read().cardstatus1().bits();

            valid_voltage = card_status >> 31 == 1
        }
    }

    Ok(card_type)
}

fn get_card_csd(card_info: &mut CardInfo, csd: [u32; 4]) {
    if card_info.card_type == CardType::SDv2HC {
        let tmp = csd[1] & 0xFF;
        let mut device_size = (tmp & 0x3F) << 16;

        let tmp = (csd[2] & 0xFFFF_0000) >> 16;
        device_size |= tmp;

        card_info.blk_number = (device_size + 1) * 1024;
        card_info.log_blk_number = card_info.blk_number;
        card_info.blk_size = 512;
        card_info.log_blk_size = card_info.blk_size;
    } else {
        let tmp = csd[1] & 0x3FF;
        let mut device_size = tmp << 2;

        let tmp = (csd[2] & 0xFF00_0000) >> 24;
        device_size |= (tmp & 0xC0) >> 6;

        let device_size_mul = (csd[2] & 0x0003_8000) >> 15;

        let rd_blk_len = (csd[1] & 0x000F_0000) >> 16;

        card_info.blk_number = (device_size + 1) * (1 << (device_size_mul + 2));
        card_info.blk_size = 1 << rd_blk_len;
        card_info.log_blk_number = card_info.blk_number * (card_info.blk_size / 512);
        card_info.log_blk_size = 512;
    }
}
