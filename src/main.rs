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

use svd_board::Hardware;

pub mod exceptions;
mod system_clock;

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
    let Hardware { rcc, gpioi, pwr, flash, .. } = hw;

    system_clock::init(rcc, pwr, flash);

    // enable gpio port i
    rcc.ahb1enr.update(|r| r.set_gpioien(true));

    // configure led pin as output pin
    gpioi.moder.update(|r| r.set_moder1(1));

    // turn led on
    gpioi.odr.update(|r| r.set_odr1(true));

    loop {
        system_clock::wait(500); // wait 0.5 seconds
        gpioi.odr.update(|r| {
            // toggle led
            let value = r.odr1();
            r.set_odr1(!value);
        });
    }
}

#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_: core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}
