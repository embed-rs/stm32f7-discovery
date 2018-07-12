use super::{FramebufferAl88, Layer, TextWriter};
use core::fmt;
use cortex_m::interrupt;
use spin::Mutex;

static STDOUT: Stdout = Stdout(Mutex::new(None));

struct Stdout<'a>(Mutex<Option<TextWriter<'a, FramebufferAl88>>>);

impl<'a> Stdout<'a> {
    fn with(&self, f: impl FnOnce(&mut Option<TextWriter<'a, FramebufferAl88>>)) {
        interrupt::free(|_| f(&mut self.0.lock()))
    }
}

pub fn init(layer: Layer<FramebufferAl88>) {
    static mut LAYER: Option<Layer<FramebufferAl88>> = None;

    STDOUT.with(|stdout| {
        let layer = unsafe { LAYER.get_or_insert_with(|| layer) };
        *stdout = Some(layer.text_writer());
    });
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
    let mut uninitialized = false;
    STDOUT.with(|stdout| {
        if let Some(ref mut stdout) = *stdout {
            stdout.write_fmt(args).unwrap();
        } else {
            uninitialized = true;
        }
    });
    if uninitialized {
        panic!("stdout uninitialized")
    }
}

pub fn is_initialized() -> bool {
    let mut initialized = false;
    STDOUT.with(|stdout| {
        initialized = stdout.is_some();
    });
    initialized
}
