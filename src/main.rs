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

    // enable gpio port c-i
    rcc.ahb1enr.update(|r| {
        r.set_gpiocen(true);
        r.set_gpioden(true);
        r.set_gpioeen(true);
        r.set_gpiofen(true);
        r.set_gpiogen(true);
        r.set_gpiohen(true);
        r.set_gpioien(true);
    });

    // configure led pin as output pin
    let led_pin = gpio.pins.i.1.take().expect("led pin already in use");
    let mut led = gpio.to_output(led_pin,
                                 gpio::Type::PushPull,
                                 gpio::Speed::Low,
                                 gpio::Resistor::NoPull);

    // turn led on
    led.set(true);

    loop {
        system_clock::wait(500); // wait 0.5 seconds
        let led_on = led.current();
        led.set(!led_on);
    }
}

#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_: core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}
