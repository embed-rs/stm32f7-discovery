//! Interrupts

#[no_mangle]
pub static INTERRUPTS: [Option<unsafe extern "C" fn()>; 97] = [None; 97];
