use super::*;
use self::command;
use board::rcc::Rcc;
use board::sdmmc::Sdmmc;
use embedded::interfaces::gpio::Gpio;

pub fn init(sdmmc: &'static mut Sdmmc, gpio: &mut Gpio, rcc: &mut Rcc) -> Result<Sd, Error> {
    init_hw(gpio, rcc)?;

    // default clock configuration
    // TODO: hardcoded
    sdmmc.clkcr.update(|clkcr| {
        clkcr.set_negedge(false);
        clkcr.set_bypass(false);
        clkcr.set_pwrsav(false);
        clkcr.set_widbus(1);
        clkcr.set_hwfc_en(false);
        clkcr.set_clkdiv(0x76);
    });

    power_on(sdmmc)?;

    Ok(Sd {
        _registers: sdmmc,
        _card_type: CardType::Sd,
    })
}

fn init_hw(gpio: &mut Gpio, rcc: &mut Rcc) -> Result<(), Error> {
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;
    use embedded::interfaces::gpio::Resistor;

    rcc.ahb1enr.update(|r| {
        r.set_gpiocen(true); // Data and clock port
        r.set_gpioden(true); // CMD port
        r.set_gpioben(true); // TODO: only needed in mmc 8bit mode
    });
    // wait for enabling
    while rcc.ahb1enr.read().gpiocen()
            && rcc.ahb1enr.read().gpioden()
            && rcc.ahb1enr.read().gpioben() {}

    // Check if a SDcard is present
    let sd_not_present = gpio.to_input((PortC, Pin13), Resistor::PullUp).unwrap();

    if sd_not_present.get() {
        return Err(Error::NoSdCard);
    }

    init_pins(gpio);

    Ok(())
}

fn init_pins(gpio: &mut Gpio) {
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;
    use embedded::interfaces::gpio::{AlternateFunction, OutputType, OutputSpeed, Resistor};

    let d0 = (PortC, Pin8);
    let d1 = (PortC, Pin9);
    let d2 = (PortC, Pin10);
    let d3 = (PortC, Pin11);
    let d4 = (PortB, Pin8);
    let d5 = (PortB, Pin9);
    let d6 = (PortC, Pin6);
    let d7 = (PortC, Pin7);

    let ck = (PortC, Pin12);

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

fn power_on(sdmmc: &mut Sdmmc) -> Result<(), Error> {
    sdmmc.clkcr.update(|clkcr| clkcr.set_clken(false));
    sdmmc.power.update(|pwr| pwr.set_pwrctrl(0x03));
    sdmmc.clkcr.update(|clkcr| clkcr.set_clken(true));

    // set sd card to idle state
    command::send_command_idle(sdmmc, 5000)?;
    command::send_command_oper_cond(sdmmc)?;

    Ok(())
}
