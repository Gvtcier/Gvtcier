use core::fmt::{self, Write};

use super::Serial;

const LOG_SIZE: usize = 4096;
static mut LOG_BUF: [u8; LOG_SIZE] = [0; LOG_SIZE];
static mut LOG_POS: usize = 0;

pub fn print(args: fmt::Arguments) {
    let mut w = SerialWriter;
    let _ = w.write_fmt(args);
}

pub fn println(args: fmt::Arguments) {
    print(args);
    Serial::write_str("\n");
    log_write("\n");
}

fn log_write(s: &str) {
    unsafe {
        for b in s.bytes() {
            if LOG_POS >= LOG_SIZE {
                LOG_POS = 0;
            }
            LOG_BUF[LOG_POS] = b;
            LOG_POS += 1;
        }
    }
}

pub fn log_dump() {
    unsafe {
        for i in 0..LOG_POS {
            Serial::write_byte(LOG_BUF[i]);
        }
        Serial::write_str("\n");
    }
}

struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Serial::write_str(s);
        log_write(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::io::Print::print(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        $crate::io::Print::println(core::format_args!($($arg)*))
    };
}
