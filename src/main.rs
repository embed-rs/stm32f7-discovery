#![feature(lang_items)]

#![no_std]
#![no_main]

// various compiler builtins such as `__aeabi_memcpy4`
extern crate compiler_builtins_snapshot;
// memcpy, memmove, etc. This needs to be below the compiler_builtins line, otherwise a linker
// error occurs (TODO: why?)
extern crate rlibc;

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
