use super::{FramebufferAl88, Layer, TextWriter};
use core::fmt;
use spin::Mutex;

static STDOUT: Mutex<Option<TextWriter<FramebufferAl88>>> = Mutex::new(None);

pub fn init(layer: Layer<FramebufferAl88>) {
    static mut LAYER: Option<Layer<FramebufferAl88>> = None;

    let mut stdout = STDOUT.lock();
    let layer = unsafe { LAYER.get_or_insert_with(|| layer) };
    *stdout = Some(layer.text_writer());
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::lcd::stdout::print(format_args!($($arg)*));
    });
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    if let Some(ref mut stdout) = *STDOUT.lock() {
        stdout.write_fmt(args).unwrap();
    } else {
        panic!("stdout uninitialized")
    }
}

pub fn with_stdout<F>(f: F)
where
    F: FnOnce(&mut Option<TextWriter<FramebufferAl88>>),
{
    f(&mut *STDOUT.lock())
}

pub unsafe fn force_unlock() {
    STDOUT.force_unlock()
}
