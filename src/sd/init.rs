use super::error::Error;
use super::{sdmmc_cmd, CardInfo, CardType, Sd};
use board::rcc::Rcc;
use board::sdmmc::Sdmmc;
use embedded::interfaces::gpio::Gpio;

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
pub fn init(sd: &mut Sd) -> Result<(), Error> {
    // Check for SD card
    if !sd.card_present() {
        return Err(Error::NoSdCard);
    }

    // Card already initialized
    if sd.card_initialized() {
        return Ok(());
    }

    // default clock configuration
    sd.sdmmc.clkcr.update(|clkcr| {
        clkcr.set_negedge(false);
        clkcr.set_bypass(false);
        clkcr.set_pwrsav(false);
        clkcr.set_widbus(0);
        clkcr.set_hwfc_en(false);
        clkcr.set_clkdiv(0x76);
    });

    let mut card_info = CardInfo::default();
    card_info.card_type = power_on(sd.sdmmc)?;

    // Let the card send the CID and enter identification process
    sdmmc_cmd::send_cid(sd.sdmmc)?;

    // Get the RCA of the card
    card_info.rca = sdmmc_cmd::set_rel_add(sd.sdmmc)?;

    sdmmc_cmd::send_csd(sd.sdmmc, u32::from(card_info.rca) << 16)?;

    let csd = [
        sd.sdmmc.resp1.read().cardstatus1(),
        sd.sdmmc.resp2.read().cardstatus2(),
        sd.sdmmc.resp3.read().cardstatus3(),
        sd.sdmmc.resp4.read().cardstatus4(),
    ];

    get_card_csd(&mut card_info, csd);

    sdmmc_cmd::sel_desel(sd.sdmmc, u32::from(card_info.rca) << 16)?;

    sd.card_info = Some(card_info);

    Ok(())
}

/// Deinitializes the SD Card.
pub fn de_init(sd: &mut Sd) {
    sd.card_info = None;

    sd.sdmmc.power.update(|pwr| pwr.set_pwrctrl(0x00));
}

/// Initializes the hardware, including the clocks and pins used by the SDMMC-Controller.
pub fn init_hw(gpio: &mut Gpio, rcc: &mut Rcc) {
    // Enable SDMMC1 clock
    rcc.apb2enr.update(|r| r.set_sdmmc1en(true));
    // Enable data and sdmmc_cmd port
    rcc.ahb1enr.update(|r| {
        r.set_gpiocen(true); // Data and clock port
        r.set_gpioden(true); // CMD port
                             // r.set_gpioben(true); // only needed in mmc 8bit mode
    });
    // wait for enabling
    while !rcc.apb2enr.read().sdmmc1en() || !rcc.ahb1enr.read().gpiocen()
        || !rcc.ahb1enr.read().gpioden()
    {}
    // || !rcc.ahb1enr.read().gpioben() {}

    init_pins(gpio);
}

fn init_pins(gpio: &mut Gpio) {
    use embedded::interfaces::gpio::Pin::*;
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::{AlternateFunction, OutputSpeed, OutputType, Resistor};

    // Data ports. For Default Bus mode only d0 is needed.
    let d0 = (PortC, Pin8);
    let d1 = (PortC, Pin9);
    let d2 = (PortC, Pin10);
    let d3 = (PortC, Pin11);
    let d4 = (PortB, Pin8);
    let d5 = (PortB, Pin9);
    let d6 = (PortC, Pin6);
    let d7 = (PortC, Pin7);

    // Clock
    let ck = (PortC, Pin12);

    // sdmmc_cmd
    let cmd = (PortD, Pin2);

    let pins = [d0, d1, d2, d3, d4, d5, d6, d7, ck, cmd];

    gpio.to_alternate_function_all(
        &pins,
        AlternateFunction::AF12,
        OutputType::PushPull,
        OutputSpeed::High,
        Resistor::PullUp,
    ).unwrap();
}

fn power_on(sdmmc: &mut Sdmmc) -> Result<CardType, Error> {
    // power up the card
    sdmmc.clkcr.update(|clkcr| clkcr.set_clken(false));
    sdmmc.power.update(|pwr| pwr.set_pwrctrl(0x03));
    sdmmc.clkcr.update(|clkcr| clkcr.set_clken(true));

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

            card_status = sdmmc.resp1.read().cardstatus1();

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

            let card_status = sdmmc.resp1.read().cardstatus1();

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
