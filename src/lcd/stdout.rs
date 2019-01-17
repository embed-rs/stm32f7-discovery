//! Initialize a LCD layer as standard output.

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

/// Initialize the passed layer as standard output.
///
/// Subsequent calls to [`print`](print) or [`println!`](println!) will then print
/// to the layer.
pub fn init(layer: Layer<FramebufferAl88>) {
    static mut LAYER: Option<Layer<FramebufferAl88>> = None;

    STDOUT.with(|stdout| {
        let layer = unsafe { LAYER.get_or_insert_with(|| layer) };
        *stdout = Some(layer.text_writer());
    });
}

/// Prints to the LCD screen, appending a newline.
///
/// The LCD stdout must be initialized. See the [`lcd::stdout::print`](lcd::stdout::print)
/// function for more information.
#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

/// Prints to the LCD screen.
///
/// The LCD stdout must be initialized. See the [`lcd::stdout::print`](lcd::stdout::print)
/// function for more information.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::lcd::stdout::print(format_args!($($arg)*));
    });
}

/// Print to the standard output.
///
/// The easiest way to use this function is through the `write!`/`writeln` macros.
///
/// Panics if the standard output is not yet initialized.
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

/// Returns whether the [`init`](init) function has already been called.
pub fn is_initialized() -> bool {
    let mut initialized = false;
    STDOUT.with(|stdout| {
        initialized = stdout.is_some();
    });
    initialized
}
