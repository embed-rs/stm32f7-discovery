#![no_std]
#![no_main]
#![feature(compiler_builtins_lib, asm)]

extern crate stm32f7_discovery as stm32f7;
extern crate compiler_builtins;
extern crate r0;

use stm32f7::{system_clock, board, embedded};

#[no_mangle]
pub unsafe extern "C" fn reset() -> ! {
    extern "C" {
        static __DATA_LOAD: u32;
        static mut __DATA_END: u32;
        static mut __DATA_START: u32;

        static mut __BSS_START: u32;
        static mut __BSS_END: u32;
    }

    // initializes the .data section (copy the data segment initializers from flash to RAM)
    r0::init_data(&mut __DATA_START, &mut __DATA_END, &__DATA_LOAD);
    // zeroes the .bss section
    r0::zero_bss(&mut __BSS_START, &__BSS_END);

    stm32f7::heap::init();

    // enable floating point unit
    let scb = stm32f7::cortex_m::peripheral::scb_mut();
    scb.cpacr.modify(|v| v | 0b1111 << 20);
    asm!("DSB; ISB;"::::"volatile"); // pipeline flush

    main(board::hw());
}

fn main(hw: board::Hardware) -> ! {
    use embedded::interfaces::gpio::{self, Gpio};


    let board::Hardware {
        rcc,
        pwr,
        flash,
        gpio_a,
        gpio_b,
        gpio_c,
        gpio_d,
        gpio_e,
        gpio_f,
        gpio_g,
        gpio_h,
        gpio_i,
        gpio_j,
        gpio_k,
        nvic,
        tim6,
        ..
    } = hw;

    let mut gpio = Gpio::new(gpio_a,
                             gpio_b,
                             gpio_c,
                             gpio_d,
                             gpio_e,
                             gpio_f,
                             gpio_g,
                             gpio_h,
                             gpio_i,
                             gpio_j,
                             gpio_k);

    system_clock::init(rcc, pwr, flash);

    // enable all gpio ports
    rcc.ahb1enr
        .update(|r| {
            r.set_gpioaen(true);
            r.set_gpioben(true);
            r.set_gpiocen(true);
            r.set_gpioden(true);
            r.set_gpioeen(true);
            r.set_gpiofen(true);
            r.set_gpiogen(true);
            r.set_gpiohen(true);
            r.set_gpioien(true);
            r.set_gpiojen(true);
            r.set_gpioken(true);
        });

    // configure led pin as output pin
    let led_pin = (gpio::Port::PortI, gpio::Pin::Pin1);
    let mut led = gpio.to_output(led_pin,
                                 gpio::OutputType::PushPull,
                                 gpio::OutputSpeed::Low,
                                 gpio::Resistor::NoPull)
        .expect("led pin already in use");

    // turn led on
    led.set(true);

    // enable timers
    rcc.apb1enr.update(|r| {
        r.set_tim6en(true);
    });

    // configure timer
    // clear update event
    tim6.sr.update(|sr| sr.set_uif(false));

    // setup timing
    tim6.psc.update(|psc| psc.set_psc(42000));
    tim6.arr.update(|arr| arr.set_arr(3000));

    // enable interrupt
    tim6.dier.update(|dier| dier.set_uie(true));
    // start the timer counter
    tim6.cr1.update(|cr1| cr1.set_cen(true));

    stm32f7::interrupts::scope(
        nvic,
        |_| {},
        |interrupt_table| {
            use stm32f7::interrupts::Priority;
            use stm32f7::interrupts::interrupt_request::InterruptRequest;

            let _ = interrupt_table.register(InterruptRequest::Tim6Dac, Priority::P1, || {
                // toggle the led
                let current_state = led.get();
                led.set(!current_state);
                // make sure the interrupt doesn't just restart again by clearing the flag
                tim6.sr.update(|sr| sr.set_uif(false));
            });

            loop {}
        },
    )
}
