// see http://embed.rs/articles/2016/semi-hosting-rust/

use core::fmt;

unsafe fn call_svc(num: usize, addr: *const ()) -> usize {
    // allocate stack space for the possible result
    let result: usize;

    // move type and argument into registers r0 and r1, then trigger
    // breakpoint 0xAB. afterwards, save a potential return value in r0
    asm!("mov r0,$1\n\t\
          mov r1,$2\n\t\
          bkpt 0xAB\n\t\
          mov $0,r0"
        : "=ri"(result)
        : "ri"(num), "ri"(addr)
        : "r0", "r1"
        : "volatile"
       );

    // return result (== r0)
    result
}

#[repr(C)]
struct SvcWriteCall {
    // the file descriptor on the host
    fd: usize,
    // pointer to data to write
    addr: *const u8,
    // length of data to write
    len: usize,
}

const SYS_WRITE: usize = 0x05;

/// Semi-hosting: `SYS_WRITE`. Writes `data` to file descriptor `fd`
/// on the host. Returns `0` on success or number of unwritten bytes
/// otherwise.
#[allow(unreachable_code, unused_variables)]
fn svc_sys_write(fd: usize, data: &[u8]) -> usize {
    return 0; // disable semi-hosting for now due to errors in the gdb script
    let args = SvcWriteCall {
        fd: fd,
        addr: data.as_ptr(),
        len: data.len(),
    };

    unsafe { call_svc(SYS_WRITE, &args as *const SvcWriteCall as *const ()) }
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::semi_hosting::print(format_args!($($arg)*));
    });
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    Stdout.write_fmt(args).unwrap();
}

static mut STDOUT_BUFFER: ([u8; 100], usize) = ([0; 100], 0);

struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        unsafe {
            for &byte in s.as_bytes() {
                STDOUT_BUFFER.0[STDOUT_BUFFER.1] = byte;
                STDOUT_BUFFER.1 += 1;
                if STDOUT_BUFFER.1 >= 100 || byte == b'\n' {
                    svc_sys_write(1, &STDOUT_BUFFER.0[..STDOUT_BUFFER.1]);
                    STDOUT_BUFFER.1 = 0;
                }
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! println_err {
    ($fmt:expr) => (print_err!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print_err!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! print_err {
    ($($arg:tt)*) => ({
        $crate::semi_hosting::print_err(format_args!($($arg)*));
    });
}

pub fn print_err(args: fmt::Arguments) {
    use core::fmt::Write;
    Stderr.write_fmt(args).unwrap();
}

struct Stderr;

impl fmt::Write for Stderr {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        svc_sys_write(2, s.as_bytes());
        Ok(())
    }
}
