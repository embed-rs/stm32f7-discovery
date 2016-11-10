#![feature(lang_items)]

#![no_std]
#![no_main]

#[no_mangle]
pub unsafe extern "C" fn reset() -> ! {
    main();
}

fn main() -> ! {
    loop {}
}

#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_: core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}
