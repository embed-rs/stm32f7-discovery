#![feature(lang_items)]
#![feature(const_fn)]

#![no_std]
#![no_main]

extern crate novemb_rs_stm32f7 as stm32f7;

// hardware register structs with accessor methods
extern crate svd_board;
// initialization routines for .data and .bss
extern crate r0;

use stm32f7::{gpio, system_clock, sdram, lcd, i2c, audio};
use svd_board::Hardware;

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
                   sai2,
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
    lcd.test_pixels();

    // i2c for audio and touch screen
    i2c::init_pins_and_clocks(rcc, &mut gpio);
    let mut i2c_3 = i2c::init(i2c3);
    i2c_3.test_1();
    i2c_3.test_2();

    // touch screen
    //assert!(touch::init_ft6x06(&mut i2c_3).is_ok());

    // sai and stereo microphone
    audio::init_sai_2_pins(&mut gpio);
    audio::init_sai_2(sai2, rcc);
    assert!(audio::init_wm8994(&mut i2c_3).is_ok());

    lcd.clear_screen();

    let mut last_led_toggle = system_clock::ticks();
    let mut last_color_change = system_clock::ticks();
    let mut button_pressed_old = false;
    loop {
        let ticks = system_clock::ticks();

        // every 0.5 seconds
        if ticks - last_led_toggle >= 500 {
            // toggle the led
            let led_current = led.current();
            led.set(!led_current);
            last_led_toggle = ticks;
        }

        let button_pressed = button.read();
        if (button_pressed && !button_pressed_old) || ticks - last_color_change >= 1000 {
            // choose a new background color
            let new_color = ((system_clock::ticks() as u32).wrapping_mul(19801)) % 0x1000000;
            lcd.set_background_color(lcd::Color::from_hex(new_color));
            last_color_change = ticks;
        }

        // poll for new audio data
        while !sai2.bsr.read().freq() {} // fifo_request_flag
        let data0 = sai2.bdr.read().data();
        while !sai2.bsr.read().freq() {} // fifo_request_flag
        let data1 = sai2.bdr.read().data();

        lcd.set_next_col(data0, data1);

        button_pressed_old = button_pressed;
    }
}

#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_args: core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}
