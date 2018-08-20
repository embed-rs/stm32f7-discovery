use core::convert::TryFrom;
use core::sync::atomic::{AtomicUsize, Ordering};
use stm32f7::stm32f7x6::{RCC, SYST};

static TICKS: AtomicUsize = AtomicUsize::new(0);
static SYSTEM_CLOCK_SPEED: AtomicUsize = AtomicUsize::new(0);
static FREQUENCY: AtomicUsize = AtomicUsize::new(0);

pub fn tick() {
    TICKS.fetch_add(1, Ordering::AcqRel);
}

pub fn ticks() -> usize {
    TICKS.load(Ordering::Acquire)
}

pub fn wait_ticks(ticks: usize) {
    let current = self::ticks();
    let desired = current + ticks;
    while self::ticks() != desired {}
}

pub fn wait_ms(ms: usize) {
    wait_ticks(ms_to_ticks(ms));
}

pub fn init(Hz(frequency): Hz, systick: &mut SYST, rcc: &RCC) {
    use cortex_m::peripheral::syst::SystClkSource;
    use stm32f7::stm32f7x6::rcc::pllcfgr::PLLPR;

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
    let reload_ticks = u32::try_from(system_clock_speed / frequency as u64).unwrap();

    SYSTEM_CLOCK_SPEED.store(system_clock_speed as usize, Ordering::Release);
    FREQUENCY.store(frequency, Ordering::Release);

    // SysTick Reload Value Register = ((25000/25) * 432) / 2 - 1 = 215_999
    // => SysTick interrupt tiggers every 1 ms
    systick.set_clock_source(SystClkSource::Core);
    systick.set_reload(reload_ticks - 1);
    systick.clear_current();
    systick.enable_counter();
}

pub fn system_clock_speed() -> Hz {
    Hz(SYSTEM_CLOCK_SPEED.load(Ordering::Acquire))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Hz(pub usize);

pub fn ticks_to_ms(ticks: usize) -> usize {
    let frequency = FREQUENCY.load(Ordering::Acquire);
    (ticks * 1000) / frequency
}

pub fn ms_to_ticks(ms: usize) -> usize {
    let frequency = FREQUENCY.load(Ordering::Acquire);
    let ticks_x1000 = frequency * ms;
    if ticks_x1000 % 1000 == 0 {
        ticks_x1000 / 1000
    } else {
        (ticks_x1000 / 1000) + 1 // round up
    }
}
