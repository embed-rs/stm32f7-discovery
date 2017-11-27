#![no_std]
#![no_main]
#![feature(compiler_builtins_lib)]

extern crate stm32f7_discovery as stm32f7;
extern crate compiler_builtins;
extern crate r0;

use stm32f7::{system_clock, board, embedded};

#[no_mangle]
pub unsafe extern "C" fn reset() -> ! {
    extern "C" {
        static __DATA_LOAD: u32;
        static __DATA_END: u32;
        static mut __DATA_START: u32;

        static mut __BSS_START: u32;
        static mut __BSS_END: u32;
    }

    let data_load = &__DATA_LOAD;
    let data_start = &mut __DATA_START;
    let data_end = &__DATA_END;

    let bss_start = &mut __BSS_START;
    let bss_end = &__BSS_END;

    // initializes the .data section
    // (copy the data segment initializers from flash to RAM)
    r0::init_data(data_start, data_end, data_load);
    // zeroes the .bss section
    r0::zero_bss(bss_start, bss_end);

    // Initialize the floating point unit
    let scb = stm32f7::cortex_m::peripheral::scb_mut();
    scb.cpacr.modify(|v| v | 0b1111 << 20);

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


    let mut last_led_toggle = system_clock::ticks();
    loop {
        let ticks = system_clock::ticks();
        // every 0.5 seconds
        if ticks - last_led_toggle >= 500 {
            // toggle the led
            let current_state = led.get();
            led.set(!current_state);
            last_led_toggle = ticks;
        }
    }
}
