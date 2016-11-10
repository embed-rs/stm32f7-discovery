#![feature(lang_items)]

#![no_std]
#![no_main]

// various compiler builtins such as `__aeabi_memcpy4`
extern crate compiler_builtins_snapshot;
// memcpy, memmove, etc. This needs to be below the compiler_builtins line, otherwise a linker
// error occurs (TODO: why?)
extern crate rlibc;
// initialization routines for .data and .bss
extern crate r0;

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

    // initializes the .data section (Copy the data segment initializers from flash to SRAM)
    r0::init_data(data_start, data_end, data_load);
    // zeroes the .bss section
    r0::zero_bss(bss_start, bss_end);

    main();
}

fn main() -> ! {
    loop {}
}

#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_: core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}
