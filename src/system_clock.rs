use board::rcc::Rcc;
use board::pwr::Pwr;
use board::flash::Flash;
use cortex_m::peripheral;

use core::sync::atomic::{AtomicUsize, Ordering};

static TICKS: AtomicUsize = AtomicUsize::new(0);

pub extern "C" fn systick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks() -> usize {
    TICKS.load(Ordering::Relaxed)
}

pub fn reset_ticks() {
    TICKS.store(0, Ordering::Relaxed);
}

pub fn wait(ms: usize) {
    let current = ticks();
    loop {
        if ticks() >= current + ms {
            break;
        }
    }
}

pub fn init(rcc: &mut Rcc, pwr: &mut Pwr, flash: &mut Flash) {
    // Enable Power Control clock
    rcc.apb1enr.update(|r| r.set_pwren(true));
    rcc.apb1enr.read(); // delay

    // Reset HSEON and HSEBYP bits before configuring the HSE
    rcc.cr
        .update(|r| {
                    r.set_hseon(false);
                    r.set_hsebyp(false);
                });
    // wait till HSE is disabled
    while rcc.cr.read().hserdy() {}
    // turn HSE on
    rcc.cr.update(|r| r.set_hseon(true));
    // wait till HSE is enabled
    while !rcc.cr.read().hserdy() {}

    // disable main PLL
    rcc.cr.update(|r| r.set_pllon(false));
    while rcc.cr.read().pllrdy() {}
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
    rcc.pllcfgr.update(|r| {
        r.set_pllsrc(true); // HSE
        r.set_pllm(25);
        r.set_plln(432); // 400 for 200 MHz, 432 for 216 MHz(don't forget to update `get_frequency`)
        r.set_pllp(0); // 0 =^= division factor 2
        r.set_pllq(9); // 8 for 200 MHz, 9 for 216 MHz
    });
    // enable main PLL
    rcc.cr.update(|r| r.set_pllon(true));
    while !rcc.cr.read().pllrdy() {}

    // enable overdrive
    pwr.cr1.update(|r| r.set_oden(true));
    while !pwr.csr1.read().odrdy() {}
    // enable overdrive switching
    pwr.cr1.update(|r| r.set_odswen(true));
    while !pwr.csr1.read().odswrdy() {}

    // Program the new number of wait states to the LATENCY bits in the FLASH_ACR register
    flash.acr.update(|r| r.set_latency(5));
    // Check that the new number of wait states is taken into account to access the Flash
    // memory by reading the FLASH_ACR register
    assert_eq!(flash.acr.read().latency(), 5);

    const NO_DIVIDE: u8 = 0;
    const SYSTEM_CLOCK_PLL: u8 = 0b10;

    // HCLK Configuration
    // HPRE = system clock not divided: AHB prescaler
    // => AHB clock frequency = system clock / 1 = 216 MHz / 1 = 216 MHz
    rcc.cfgr.update(|r| r.set_hpre(NO_DIVIDE));
    // SYSCLK Configuration
    rcc.cfgr.update(|r| r.set_sw(SYSTEM_CLOCK_PLL));
    while rcc.cfgr.read().sws() != SYSTEM_CLOCK_PLL {}

    const DIVIDE_2: u8 = 0b100;
    const DIVIDE_4: u8 = 0b101;

    // PCLK1 Configuration
    // PPRE1: APB Low-speed prescaler (APB1)
    // => APB low-speed clock frequency = AHB clock / 4 = 216 Mhz / 4 = 54 MHz
    // FIXME: Frequency should not exceed 45 MHz
    rcc.cfgr.update(|r| r.set_ppre1(DIVIDE_4));
    // PCLK2 Configuration
    // PPRE2: APB high-speed prescaler (APB2)
    // => APB high-speed clock frequency = AHB clock / 2 = 216 Mhz / 2 = 108 MHz
    // FIXME: Frequency should not exceed 90 MHz
    rcc.cfgr.update(|r| r.set_ppre2(DIVIDE_2));


    let systick = unsafe { peripheral::syst_mut() };

    let pll_cfgr = rcc.pllcfgr.read();
    let pllm = u32::from(pll_cfgr.pllm());
    let plln = u32::from(pll_cfgr.plln());
    let pllp = u32::from(pll_cfgr.pllp() + 1) * 2;
    // SysTick Reload Value Register = ((25000/25) * 432) / 2 - 1 = 215_999
    // => SysTick interrupt tiggers every 1 ms
    systick.rvr.write((((25 * 1000) / pllm) * plln) / pllp - 1); // hse runs at 25 MHz
    systick.cvr.write(0); // clear
    systick.csr.write(0b111); // CLKSOURCE | TICKINT | ENABLE

    reset_ticks();
}

pub fn get_frequency() -> u32 {
    216_000_000 // 216 MHz
}

pub fn get_ahb_frequency() -> u32 {
    get_frequency() / 1 //216 MHz
}

pub fn get_apb1_frequency() -> u32 {
    get_ahb_frequency() / 4 // 54 MHz
}

pub fn get_apb2_frequency() -> u32 {
    get_ahb_frequency() / 2 // 108 MHz
}