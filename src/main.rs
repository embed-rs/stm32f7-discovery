#![feature(lang_items)]

#![no_std]

fn main() {}

#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_: core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}
