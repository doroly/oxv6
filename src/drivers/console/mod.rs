//! Line-buffered console built on top of the UART driver.
//!
//! Input is canonical: a line becomes readable after Enter. The UART interrupt
//! path performs echo and basic editing without waiting for a reader.

pub(crate) mod uart;

use crate::sync::SpinLock;
use core::fmt::{self, Write};

const INPUT_CAPACITY: usize = 128;
const BACKSPACE: u8 = 0x08;
const CTRL_U: u8 = 0x15;

struct Console {
    buf: [u8; INPUT_CAPACITY],
    r: usize,
    w: usize,
    e: usize,
}

impl Console {
    const fn new() -> Self {
        Self {
            buf: [0; INPUT_CAPACITY],
            r: 0,
            w: 0,
            e: 0,
        }
    }

    #[inline]
    const fn next(index: usize) -> usize {
        (index + 1) % INPUT_CAPACITY
    }

    fn erase_last(&mut self) {
        if self.e != self.w {
            self.e = (self.e + INPUT_CAPACITY - 1) % INPUT_CAPACITY;
            uart::putchar(BACKSPACE);
            uart::putchar(b' ');
            uart::putchar(BACKSPACE);
        }
    }

    fn input(&mut self, mut ch: u8) {
        if ch == b'\r' {
            ch = b'\n';
        }
        match ch {
            BACKSPACE | 0x7f => self.erase_last(),
            CTRL_U => {
                while self.e != self.w {
                    self.erase_last();
                }
            }
            b'\n' => {
                self.buf[self.e] = ch;
                self.e = Self::next(self.e);
                self.w = self.e;
                uart::putchar(b'\r');
                uart::putchar(b'\n');
            }
            b'\t' | 0x20..=0x7e => {
                let next = Self::next(self.e);
                if next == self.r {
                    // The bounded buffer is full: publish the partial line.
                    self.w = self.e;
                } else {
                    self.buf[self.e] = ch;
                    self.e = next;
                    uart::putchar(ch);
                }
            }
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn read(&mut self, dst: &mut [u8]) -> usize {
        let mut count = 0;
        while count < dst.len() && self.r != self.w {
            let ch = self.buf[self.r];
            self.r = Self::next(self.r);
            dst[count] = ch;
            count += 1;
            if ch == b'\n' {
                break;
            }
        }
        count
    }
}

static CONSOLE: SpinLock<Console> = SpinLock::new("console", Console::new());

/// Called by the UART receive interrupt for every received byte.
pub(crate) fn intr(ch: u8) {
    CONSOLE.lock().input(ch);
}

/// Copies a completed line (or part of one) without blocking.
#[allow(dead_code)]
pub(crate) fn read(dst: &mut [u8]) -> usize {
    CONSOLE.lock().read(dst)
}

pub(crate) fn write(bytes: &[u8]) {
    for &byte in bytes {
        uart::putchar(byte);
    }
}

struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(s.as_bytes());
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    ConsoleWriter.write_fmt(args).unwrap();
}
