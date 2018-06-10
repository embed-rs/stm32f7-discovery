use core::convert::TryFrom;
use stm32f7x6::{FLASH, PWR, RCC, SYST};

pub fn init_system_clock_216mhz(rcc: &mut RCC, pwr: &mut PWR, flash: &mut FLASH) {
    // enable power control clock
    rcc.apb1enr.modify(|_, w| w.pwren().enabled());
    rcc.apb1enr.read(); // delay

    // reset HSEON and HSEBYP bits before configuring HSE
    rcc.cr.modify(|_, w| {
        w.hseon().clear_bit();
        w.hsebyp().clear_bit();
        w
    });
    // wait until HSE is disabled
    while rcc.cr.read().hserdy().bit_is_set() {}
    // turn HSE on
    rcc.cr.modify(|_, w| w.hseon().set_bit());
    // wait until HSE is enabled
    while rcc.cr.read().hserdy().bit_is_clear() {}

    // Configure the main PLL clock source, multiplication and division factors.
    // HSE is used as clock source. HSE runs at 25 MHz.
    // PLLM = 25: Division factor for the main PLLs (PLL, PLLI2S and PLLSAI) input clock
    // VCO input frequency = PLL input clock frequency / PLLM with 2 ≤ PLLM ≤ 63
    // => VCO input frequency = 25_000 kHz / 25 = 1_000 kHz = 1 MHz
    // PPLM = 432: Main PLL (PLL) multiplication factor for VCO
    // VCO output frequency = VCO input frequency × PLLN with 50 ≤ PLLN ≤ 432
    // => VCO output frequency 1 Mhz * 432 = 432 MHz
    // PPLQ = 0 =^= division factor 2: Main PLL (PLL) division factor for main system clock
    // PLL output clock frequency = VCO frequency / PLLP with PLLP = 2, 4, 6, or 8
    // => PLL output clock frequency = 432 MHz / 2 = 216 MHz
    rcc.pllcfgr.modify(|_, w| {
        w.pllsrc().hse();
        w.pllp().div2();
        unsafe {
            // Frequency = ((TICKS / pllm) * plln) / pllp
            // HSE runs at 25 MHz
            w.pllm().bits(25);
            w.plln().bits(432); // 400 for 200 MHz, 432 for 216 MHz
            w.pllq().bits(9); // 8 for 200 MHz, 9 for 216 MHz
        }
        w
    });
    // enable main PLL
    rcc.cr.modify(|_, w| w.pllon().set_bit());
    while rcc.cr.read().pllrdy().bit_is_clear() {}

    // enable overdrive
    pwr.cr1.modify(|_, w| w.oden().set_bit());
    while pwr.csr1.read().odrdy().bit_is_clear() {}
    // enable overdrive switching
    pwr.cr1.modify(|_, w| w.odswen().set_bit());
    while pwr.csr1.read().odswrdy().bit_is_clear() {}

    // Program the new number of wait states to the LATENCY bits in the FLASH_ACR register
    flash.acr.modify(|_, w| unsafe { w.latency().bits(5) });
    // Check that the new number of wait states is taken into account to access the Flash
    // memory by reading the FLASH_ACR register
    assert_eq!(flash.acr.read().latency().bits(), 5);

    // HCLK Configuration
    // HPRE = system clock not divided: AHB prescaler
    // => AHB clock frequency = system clock / 1 = 216 MHz / 1 = 216 MHz
    rcc.cfgr.modify(|_, w| w.hpre().div1());
    // SYSCLK Configuration
    rcc.cfgr.modify(|_, w| w.sw().pll());
    while !rcc.cfgr.read().sws().is_pll() {}

    // PCLK1 Configuration
    // PPRE1: APB Low-speed prescaler (APB1)
    // => APB low-speed clock frequency = AHB clock / 4 = 216 Mhz / 4 = 54 MHz
    // FIXME: Frequency should not exceed 45 MHz
    rcc.cfgr.modify(|_, w| w.ppre1().div4());
    // PCLK2 Configuration
    // PPRE2: APB high-speed prescaler (APB2)
    // => APB high-speed clock frequency = AHB clock / 2 = 216 Mhz / 2 = 108 MHz
    // FIXME: Frequency should not exceed 90 MHz
    rcc.cfgr.modify(|_, w| w.ppre2().div2());
}

pub fn init_systick(Hz(frequency): Hz, systick: &mut SYST, rcc: &RCC) {
    use cortex_m::peripheral::syst::SystClkSource;
    use stm32f7x6::rcc::pllcfgr::PLLPR;

    let pll_cfgr = rcc.pllcfgr.read();
    let pllm = u64::from(pll_cfgr.pllm().bits());
    let plln = u64::from(pll_cfgr.plln().bits());
    let pllp = match pll_cfgr.pllp() {
        PLLPR::DIV2 => 2,
        PLLPR::DIV4 => 4,
        PLLPR::DIV6 => 6,
        PLLPR::DIV8 => 8,
    };

    let system_clock_speed = (((25 * 1000 * 1000) / pllm) * plln) / pllp; // HSE runs at 25 MHz
    let reload_ticks = u32::try_from(system_clock_speed / frequency).unwrap();

    // SysTick Reload Value Register = ((25000/25) * 432) / 2 - 1 = 215_999
    // => SysTick interrupt tiggers every 1 ms
    systick.set_clock_source(SystClkSource::External);
    systick.set_reload(reload_ticks - 1);
    systick.clear_current();
    systick.enable_counter();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Hz(pub u64);
