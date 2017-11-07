use super::{Sd, CardType, command};
use super::error::Error;
use board::rcc::Rcc;
use board::sdmmc::Sdmmc;
use embedded::interfaces::gpio::Gpio;

/// Initializes the SD card. This includes the initialization of the pins and clocks that the card
/// needs to operate. If the initialization succeeds an SD card struct is returned.
///
/// # Errors
///
/// The initialization returns an error if something in the process went wrong. Most of the time
/// this should only be the Error `NoSdCard`. Other errors are an indication that some clock or pin
/// could not be activated correctly.
pub fn init(sdmmc: &'static mut Sdmmc, gpio: &mut Gpio, rcc: &mut Rcc) -> Result<Sd, Error> {
    // Check for SD card
    sd_card_present(gpio, rcc)?;

    // Initialize clock and pins
    init_hw(gpio, rcc)?;

    // default clock configuration
    sdmmc.clkcr.update(|clkcr| {
        clkcr.set_negedge(false);
        clkcr.set_bypass(false);
        clkcr.set_pwrsav(false);
        clkcr.set_widbus(1);
        clkcr.set_hwfc_en(false);
        clkcr.set_clkdiv(0x76);
    });

    let card_type = power_on(sdmmc)?;

    command::send_cmd_send_cid(sdmmc)?;

    let rca = command::send_cmd_set_rel_add(sdmmc)?;

    Ok(Sd {
        _sdmmc: sdmmc,
        card_type: card_type,
        rca: rca,
    })
}

fn sd_card_present(gpio: &mut Gpio, rcc: &mut Rcc) -> Result<(), Error> {
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;
    use embedded::interfaces::gpio::Resistor;

    rcc.ahb1enr.update(|r| r.set_gpiocen(true));
    // wait for enabling
    while !rcc.ahb1enr.read().gpiocen() {}

    let sd_not_present = gpio.to_input((PortC, Pin13), Resistor::PullUp).unwrap();
    if sd_not_present.get() {
        return Err(Error::NoSdCard);
    }

    Ok(())
}

fn init_hw(gpio: &mut Gpio, rcc: &mut Rcc) -> Result<(), Error> {
    // Enable SDMMC1 clock
    rcc.apb2enr.update(|r| r.set_sdmmc1en(true));
    // Enable data and command port
    rcc.ahb1enr.update(|r| {
        r.set_gpiocen(true); // Data and clock port
        r.set_gpioden(true); // CMD port
        // r.set_gpioben(true); // only needed in mmc 8bit mode
    });
    // wait for enabling
    while !rcc.apb2enr.read().sdmmc1en()
            || !rcc.ahb1enr.read().gpiocen()
            || !rcc.ahb1enr.read().gpioden() {}
            // || !rcc.ahb1enr.read().gpioben() {}

    init_pins(gpio);

    Ok(())
}

fn init_pins(gpio: &mut Gpio) {
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;
    use embedded::interfaces::gpio::{AlternateFunction, OutputType, OutputSpeed, Resistor};

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

    // Command
    let cmd = (PortD, Pin2);

    let pins = [d0,
                d1,
                d2,
                d3,
                d4,
                d5,
                d6,
                d7,
                ck,
                cmd];

    gpio.to_alternate_function_all(&pins,
                                   AlternateFunction::AF12,
                                   OutputType::PushPull,
                                   OutputSpeed::High,
                                   Resistor::PullUp).unwrap();
}

fn power_on(sdmmc: &mut Sdmmc) -> Result<CardType, Error> {
    // power up the card
    sdmmc.clkcr.update(|clkcr| clkcr.set_clken(false));
    sdmmc.power.update(|pwr| pwr.set_pwrctrl(0x03));
    sdmmc.clkcr.update(|clkcr| clkcr.set_clken(true));

    let mut card_type = CardType::SDv1;

    // set sd card to idle state
    command::send_cmd_idle(sdmmc, 5000)?;

    // get Card version and operation voltage
    let mut count = 0;
    let max_volt_trial = 0xFFFF;
    let mut valid_voltage = false;
    if command::send_cmd_oper_cond(sdmmc).is_ok() {
        let mut card_status = 0;
        // voltage trial for card V2
        while !valid_voltage {
            if count == max_volt_trial {
                return Err(Error::InvalidVoltrange)
            }
            count += 1;

            // Send CMD55, needed for next CMD. println!("Send CMD55");
            command::send_cmd_app(sdmmc, 0)?;

            // Send ACMD41. 0x40..0 for high capacity. println!("Send ACMD41");
            command::send_cmd_app_oper(sdmmc, 0x4000_0000)?;

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
                return Err(Error::InvalidVoltrange)
            }
            count += 1;

            // Send CMD55, needed for next CMD. println!("Send CMD55");
            command::send_cmd_app(sdmmc, 0)?;

            // Send ACMD41. 0x0 for standard capacity. println!("Send ACMD41");
            command::send_cmd_app_oper(sdmmc, 0x0)?;

            let card_status = sdmmc.resp1.read().cardstatus1();

            valid_voltage = card_status >> 31 == 1
        }
    }

    Ok(card_type)
}
