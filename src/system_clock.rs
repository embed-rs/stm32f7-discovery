//! Provides initialization and time-keeping functions for the system clock (`systick`).

use core::convert::TryFrom;
use core::sync::atomic::{AtomicUsize, Ordering};
use stm32f7::stm32f7x6::{RCC, SYST};

static TICKS: AtomicUsize = AtomicUsize::new(0);
static SYSTEM_CLOCK_SPEED: AtomicUsize = AtomicUsize::new(0);
static FREQUENCY: AtomicUsize = AtomicUsize::new(0);

/// Increases the global tick count by 1.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::AcqRel);
}

/// Returns the current global tick count.
pub fn ticks() -> usize {
    TICKS.load(Ordering::Acquire)
}

/// Returns the elapsed milliseconds since [`tick()`] was first called.
///
/// [`tick()`]: self::tick
pub fn ms() -> usize {
    ticks_to_ms(ticks())
}

/// Wait for the specified number of ticks.
///
/// This function spins the thread in a while loop until the [`tick()`] function was invoked
/// `ticks` times.
pub fn wait_ticks(ticks: usize) {
    let current = self::ticks();
    let desired = current + ticks;
    while self::ticks() != desired {}
}

/// Wait for the specified number of milliseconds.
///
/// This function spins the thread in a while loop until the specified number of milliseconds
/// have passed. This function is based on [`wait_ticks`] and [`ms_to_ticks`].
///
/// [`wait_ticks`]: self::wait_ticks
/// [`ms_to_ticks`]: self::ms_to_ticks
pub fn wait_ms(ms: usize) {
    wait_ticks(ms_to_ticks(ms));
}

/// Initializes the system clock (systick) of the stm32f7-discovery board to the specified
/// frequency.
///
/// After calling this function, the interrupt handler for the systick interrupt should call
/// [`tick()`] on each invocation to update the global tick counter in this module.
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
    assert!(
        reload_ticks < 0x0100_0000,
        "Systick frequency is too low for the SysTick RVR register. \
         The minimum frequency for the current system frequency is {}Hz",
        system_clock_speed as f32 / 0x0100_0000 as f32
    );

    SYSTEM_CLOCK_SPEED.store(system_clock_speed as usize, Ordering::Release);
    FREQUENCY.store(frequency, Ordering::Release);

    // SysTick Reload Value Register = ((25000/25) * 432) / 2 - 1 = 215_999
    // => SysTick interrupt tiggers every 1 ms
    systick.set_clock_source(SystClkSource::Core);
    systick.set_reload(reload_ticks - 1);
    systick.clear_current();
    systick.enable_counter();
}

/// Returns the frequency of the system clock.
///
/// This is the frequency that was passed to [`init`].
///
/// [`init`]: self::init
pub fn system_clock_speed() -> Hz {
    Hz(SYSTEM_CLOCK_SPEED.load(Ordering::Acquire))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
/// A frequency in Hz.
pub struct Hz(pub usize);

/// Translates the passed tick number to a number of milliseconds.
///
/// Depends on the [`system_clock_speed`](self::system_clock_speed).
pub fn ticks_to_ms(ticks: usize) -> usize {
    let frequency = FREQUENCY.load(Ordering::Acquire);
    (ticks * 1000) / frequency
}

/// Translates the passed number of milliseconds to a number of ticks.
///
/// Depends on the [`system_clock_speed`](self::system_clock_speed).
pub fn ms_to_ticks(ms: usize) -> usize {
    let frequency = FREQUENCY.load(Ordering::Acquire);
    let ticks_x1000 = frequency * ms;
    if ticks_x1000 % 1000 == 0 {
        ticks_x1000 / 1000
    } else {
        (ticks_x1000 / 1000) + 1 // round up
    }
}
