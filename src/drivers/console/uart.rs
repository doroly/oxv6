//! Console I/O for the QEMU `virt` machine UART.
//!
//! Exposes a minimal serial interface for early kernel diagnostics and debugging using an
//! NS16550-compatible UART mapped at physical address `0x1000_0000`.

use core::fmt::{self, Write};
use core::ptr;

/// Base MMIO physical address for the 16550A UART controller.
const UART_BASE: usize = 0x1000_0000;

/// Register offsets.
const RHR: usize = 0; // Receive Holding Register (read-only)
const THR: usize = 0; // Transmit Holding Register (write-only)
const IER: usize = 1; // Interrupt Enable Register
const LSR: usize = 5; // Line Status Register

/// Register bit masks.
const IER_RX_ENABLE: u8 = 1 << 0; // Enable Receiver Data Available Interrupt
const LSR_RX_READY: u8 = 1 << 0; // Receiver Data Ready Flag

/// Computes a raw pointer to a UART register at the given offset.
#[inline]
fn reg_ptr(offset: usize) -> *mut u8 {
    (UART_BASE + offset) as *mut u8
}

/// Zero-sized type implementing [`Write`] to enable formatted output (`write!`/`writeln!`).
pub(crate) struct UartWriter;

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            putchar(byte);
        }
        Ok(())
    }
}

/// Internal helper function used by `print!` and `println!` macros.
pub(crate) fn _print(args: fmt::Arguments) {
    let _ = UartWriter.write_fmt(args);
}

/// Prints formatted text to the UART console.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::drivers::uart::_print(format_args!($($arg)*))
    };
}

/// Prints formatted text to the UART console with an appended newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\n"))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print!(concat!($fmt, "\n"), $($arg)*)
    };
}

/// Initializes UART interrupts for receiving characters.
pub(crate) fn init() {
    unsafe {
        ptr::write_volatile(reg_ptr(IER), IER_RX_ENABLE);
    }
}

/// Transmits a single byte to the UART serial port.
#[inline]
pub(crate) fn putchar(ch: u8) {
    unsafe {
        ptr::write_volatile(reg_ptr(THR), ch);
    }
}

/// Attempts to read a single byte from the UART receiver buffer.
///
/// Returns `Some(u8)` if a character is available, or `None` otherwise.
#[inline]
pub(crate) fn getchar() -> Option<u8> {
    unsafe {
        let lsr = ptr::read_volatile(reg_ptr(LSR));
        if (lsr & LSR_RX_READY) != 0 {
            Some(ptr::read_volatile(reg_ptr(RHR)))
        } else {
            None
        }
    }
}

/// Handles UART receive interrupts.
///
/// Drains all available incoming bytes from the hardware buffer and echoes them back.
pub(crate) fn handle_interrupt() {
    while let Some(ch) = getchar() {
        putchar(ch);
    }
}
