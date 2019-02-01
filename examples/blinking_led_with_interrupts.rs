#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc_cortex_m;
extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting;
extern crate stm32f7;
extern crate stm32f7_discovery;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout as AllocLayout;
use core::panic::PanicInfo;
use rt::{entry, exception};
use stm32f7::stm32f7x6::{CorePeripherals, Peripherals};
use stm32f7_discovery::{
    gpio::{GpioPort, OutputPin},
    init,
    interrupts::{self, InterruptRequest, Priority},
    system_clock::{self, Hz},
};

const HEAP_SIZE: usize = 50 * 1024; // in bytes

#[entry]
fn main() -> ! {
    let core_peripherals = CorePeripherals::take().unwrap();
    let mut systick = core_peripherals.SYST;
    let mut nvic = core_peripherals.NVIC;

    let peripherals = Peripherals::take().unwrap();
    let mut rcc = peripherals.RCC;
    let mut pwr = peripherals.PWR;
    let mut flash = peripherals.FLASH;
    let mut tim6 = peripherals.TIM6;
    let mut nvic_stir = peripherals.NVIC_STIR;

    init::init_system_clock_216mhz(&mut rcc, &mut pwr, &mut flash);
    init::enable_gpio_ports(&mut rcc);

    let gpio_a = GpioPort::new(peripherals.GPIOA);
    let gpio_b = GpioPort::new(peripherals.GPIOB);
    let gpio_c = GpioPort::new(peripherals.GPIOC);
    let gpio_d = GpioPort::new(peripherals.GPIOD);
    let gpio_e = GpioPort::new(peripherals.GPIOE);
    let gpio_f = GpioPort::new(peripherals.GPIOF);
    let gpio_g = GpioPort::new(peripherals.GPIOG);
    let gpio_h = GpioPort::new(peripherals.GPIOH);
    let gpio_i = GpioPort::new(peripherals.GPIOI);
    let gpio_j = GpioPort::new(peripherals.GPIOJ);
    let gpio_k = GpioPort::new(peripherals.GPIOK);
    let mut pins = init::pins(
        gpio_a, gpio_b, gpio_c, gpio_d, gpio_e, gpio_f, gpio_g, gpio_h, gpio_i, gpio_j, gpio_k,
    );

    // configure the systick timer 20Hz (20 ticks per second)
    init::init_systick(Hz(20), &mut systick, &rcc);
    systick.enable_interrupt();

    // turn led on
    pins.led.set(true);

    // enable timers
    rcc.apb1enr.modify(|_, w| w.tim6en().enabled());

    // configure timer
    // clear update event
    tim6.sr.modify(|_, w| w.uif().clear_bit());

    // setup timing
    tim6.psc.modify(|_, w| unsafe { w.psc().bits(42000) });
    tim6.arr.modify(|_, w| unsafe { w.arr().bits(3000) });

    // enable interrupt
    tim6.dier.modify(|_, w| w.uie().set_bit());
    // start the timer counter
    tim6.cr1.modify(|_, w| w.cen().set_bit());

    // The interrupt module needs an allocator for its dynamic interrupt table.
    unsafe { ALLOCATOR.init(rt::heap_start() as usize, HEAP_SIZE) }

    interrupts::scope(
        &mut nvic,
        &mut nvic_stir,
        |_| {},
        |interrupt_table| {
            let _ = interrupt_table.register(InterruptRequest::TIM6_DAC, Priority::P1, || {
                pins.led.toggle();
                let tim = &mut tim6;
                // make sure the interrupt doesn't just restart again by clearing the flag
                tim.sr.modify(|_, w| w.uif().clear_bit());
            });

            loop {}
        },
    )
}

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[exception]
fn SysTick() {
    system_clock::tick();
}

// define what happens in an Out Of Memory (OOM) condition
#[alloc_error_handler]
fn rust_oom(_: AllocLayout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;
    use cortex_m::asm;
    use cortex_m_semihosting::hio;

    if let Ok(mut hstdout) = hio::hstdout() {
        let _ = writeln!(hstdout, "{}", info);
    }

    // OK to fire a breakpoint here because we know the microcontroller is connected to a debugger
    asm::bkpt();

    loop {}
}
