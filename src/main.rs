#![feature(lang_items)]
#![feature(const_fn)]

#![no_std]
#![no_main]

// various compiler builtins such as `__aeabi_memcpy4`
extern crate compiler_builtins_snapshot;
// memcpy, memmove, etc. This needs to be below the compiler_builtins line, otherwise a linker
// error occurs (TODO: why?)
extern crate rlibc;
// initialization routines for .data and .bss
extern crate r0;
// hardware register structs with accessor methods
extern crate svd_board;
// low level access to the cortex-m cpu
extern crate cortex_m;
// volatile wrapper types
extern crate volatile;

use svd_board::Hardware;

pub mod exceptions;
mod system_clock;
mod gpio;
mod sdram;
mod lcd;
mod i2c;

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

    // initializes the .data section (copy the data segment initializers from flash to RAM)
    r0::init_data(data_start, data_end, data_load);
    // zeroes the .bss section
    r0::zero_bss(bss_start, bss_end);

    main(svd_board::hw());
}

fn main(hw: Hardware) -> ! {
    let Hardware { rcc,
                   pwr,
                   flash,
                   fmc,
                   ltdc,
                   gpioa,
                   gpiob,
                   gpioc,
                   gpiod,
                   gpioe,
                   gpiof,
                   gpiog,
                   gpioh,
                   gpioi,
                   gpioj,
                   gpiok,
                   i2c3,
                   .. } = hw;

    let mut gpio = unsafe {
        gpio::GpioController::new(gpioa,
                                  gpiob,
                                  gpioc,
                                  gpiod,
                                  gpioe,
                                  gpiof,
                                  gpiog,
                                  gpioh,
                                  gpioi,
                                  gpioj,
                                  gpiok)
    };

    system_clock::init(rcc, pwr, flash);

    // enable all gpio ports
    rcc.ahb1enr.update(|r| {
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
    let led_pin = gpio.pins.i.1.take().expect("led pin already in use");
    let mut led = gpio.to_output(led_pin,
                                 gpio::Type::PushPull,
                                 gpio::Speed::Low,
                                 gpio::Resistor::NoPull);

    // turn led on
    led.set(true);

    let button_pin = gpio.pins.i.11.take().expect("button pin already in use");
    let button = gpio.to_input(button_pin, gpio::Resistor::NoPull);

    // init sdram (needed for display buffer)
    sdram::init(rcc, fmc, &mut gpio);

    // lcd controller
    let mut lcd = lcd::init(ltdc, rcc, &mut gpio);
    lcd.clear_screen();
    system_clock::wait(1000);
    lcd.test_pixels();

    // i2c
    i2c::init_pins_and_clocks(rcc, &mut gpio);
    let mut i2c_3 = i2c::init(i2c3);
    i2c_3.test_1();
    i2c_3.test_2();

    loop {
        let ticks = system_clock::ticks();

        // every 0.5 seconds
        if ticks % 500 == 0 {
            // toggle the led
            let led_on = led.current();
            led.set(!led_on);
        }

        if button.read() || ticks % 1000 == 0 {
            // choose a new background color
            let new_color = ((system_clock::ticks() as u32).wrapping_mul(19801)) % 0x1000000;
            lcd.set_background_color(lcd::Color::from_hex(new_color));
        }
    }
}

#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_: core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}
