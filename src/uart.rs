//! Console I/O for the QEMU `virt` machine UART.
//!
//! The driver exposes a minimal serial interface for early kernel diagnostics and debugging. It
//! targets the common NS16550-compatible UART mapped at `0x1000_0000` on the QEMU virt platform.

/// Sends a single byte to the UART transmitter.
#[inline]
pub(crate) fn putchar(ch: u8) {
    let uart = 0x1000_0000 as *mut u8;

    unsafe {
        uart.write_volatile(ch);
    }
}

/// Reads a single byte from the UART receiver in blocking mode.
///
/// This function waits until the receive-data-ready bit is set in the UART line status register
/// before returning the next byte.
pub(crate) fn getchar() -> u8 {
    let uart = 0x1000_0000 as *mut u8;

    unsafe {
        while uart.add(5).read_volatile() & 1 == 0 {}
        uart.read_volatile()
    }
}

/// Prints a Rust string to the UART console.
#[inline]
pub(crate) fn print_str(s: &str) {
    for byte in s.bytes() {
        putchar(byte);
    }
}

/// Prints an integer value in hexadecimal format, prefixed with `0x`.
pub(crate) fn print_hex(mut value: usize) {
    print_str("0x");

    if value == 0 {
        putchar(b'0');
        return;
    }

    let mut buffer = [0u8; 16];
    let mut index = 0;

    while value > 0 {
        let digit = (value & 0xF) as u8;
        buffer[index] = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + (digit - 10)
        };
        value >>= 4;
        index += 1;
    }

    while index > 0 {
        index -= 1;
        putchar(buffer[index]);
    }
}

/// A small echo loop used for basic UART validation and interactive debugging.
#[allow(dead_code)]
pub(crate) fn test_uart() {
    print_str("Hello, RVOS from Rust!\n");
    print_str("RVOS Console Active. Type something:\n");

    loop {
        let ch = getchar();

        if ch == b'\r' {
            putchar(b'\r');
            putchar(b'\n');
        } else {
            putchar(ch);
        }
    }
}
