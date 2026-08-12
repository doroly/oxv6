//! UART driver for the RISC-V QEMU virt machine.
//!
//! This module provides basic console input/output through
//! the 16550 UART device mapped at 0x10000000.

/// Write a single byte to the UART console.
///
/// The UART device uses memory-mapped I/O (MMIO). This function
/// performs a volatile write to ensure the compiler does not
/// optimize away the hardware access.
pub(crate) fn putchar(ch: u8) {
    // QEMU virt machine UART base address.
    let uart = 0x1000_0000 as *mut u8;

    unsafe {
        uart.write_volatile(ch);
    }
}

/// Read a single byte from the UART console.
///
/// This function uses blocking mode and waits until the UART
/// receives a new character.
pub(crate) fn getchar() -> u8 {
    // QEMU virt machine UART base address.
    let uart = 0x1000_0000 as *mut u8;

    unsafe {
        // Line Status Register (LSR) is located at offset 5.
        // Bit 0 indicates whether received data is available.
        while uart.add(5).read_volatile() & 1 == 0 {}

        // Read received byte from Receiver Buffer Register (RBR).
        uart.read_volatile()
    }
}

/// Print a string to the UART console.
pub(crate) fn print_str(s: &str) {
    for byte in s.bytes() {
        putchar(byte);
    }
}

/// Print an integer value in hexadecimal format.
///
/// This function is mainly used for displaying memory addresses.
pub(crate) fn print_hex(mut val: usize) {
    // Print hexadecimal prefix.
    print_str("0x");

    // Handle zero specially.
    if val == 0 {
        putchar(b'0');
        return;
    }

    let mut buf = [0u8; 16];
    let mut i = 0;

    // Extract hexadecimal digits.
    while val > 0 {
        let digit = (val & 0xF) as u8;

        buf[i] = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + (digit - 10)
        };

        val >>= 4;
        i += 1;
    }

    // Print digits in reverse order.
    while i > 0 {
        i -= 1;
        putchar(buf[i]);
    }
}

/// Test UART input and output.
///
/// Prints a welcome message and echoes user input
/// back to the console.
pub(crate) fn test_uart() {
    print_str("Hello, RVOS from Rust!\n");
    print_str("RVOS Console Active. Type something:\n");

    loop {
        // Wait for user input.
        let ch = getchar();

        // Handle Enter key.
        if ch == b'\r' {
            putchar(b'\r');
            putchar(b'\n');
        } else {
            // Echo received character.
            putchar(ch);
        }
    }
}
