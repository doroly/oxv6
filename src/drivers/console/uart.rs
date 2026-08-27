//! Console I/O for the QEMU `virt` machine UART.
//!
//! Provides a queued, interrupt-driven UART backend with polling output during
//! bootstrap, when interrupts cannot yet drain the transmit queue.

use crate::sync::SpinLock;
use core::fmt;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

const UART_BASE: usize = 0x1000_0000;

const RHR: usize = 0;
const THR: usize = 0;
const IER: usize = 1;
const IIR: usize = 2;
const LSR: usize = 5;

const IER_RX_ENABLE: u8 = 1 << 0;
const IER_TX_ENABLE: u8 = 1 << 1;
const LSR_RX_READY: u8 = 1 << 0;
const LSR_TX_IDLE: u8 = 1 << 5;
const IIR_NO_PENDING: u8 = 1 << 0;
const IIR_ID_MASK: u8 = 0x0e;
const IIR_TX_EMPTY: u8 = 0x02;
const UART_TX_CAPACITY: usize = 32;

#[inline]
fn reg_ptr(offset: usize) -> *mut u8 {
    (UART_BASE + offset) as *mut u8
}

pub struct Uart {
    tx: [u8; UART_TX_CAPACITY],
    tx_r: usize,
    tx_w: usize,
}

impl Uart {
    pub const fn new() -> Self {
        Self {
            tx: [0; UART_TX_CAPACITY],
            tx_r: 0,
            tx_w: 0,
        }
    }

    fn putc_raw(&self, ch: u8) {
        unsafe {
            while (ptr::read_volatile(reg_ptr(LSR)) & LSR_TX_IDLE) == 0 {
                core::hint::spin_loop();
            }
            ptr::write_volatile(reg_ptr(THR), ch);
        }
    }

    #[inline]
    fn tx_next(index: usize) -> usize {
        (index + 1) % UART_TX_CAPACITY
    }

    /// Moves queued bytes into the hardware while THR is empty.
    /// The caller holds `WRITER`.
    fn transmit(&mut self) {
        unsafe {
            while self.tx_r != self.tx_w && (ptr::read_volatile(reg_ptr(LSR)) & LSR_TX_IDLE) != 0 {
                ptr::write_volatile(reg_ptr(THR), self.tx[self.tx_r]);
                self.tx_r = Self::tx_next(self.tx_r);
            }
            let ier = if self.tx_r == self.tx_w {
                IER_RX_ENABLE
            } else {
                IER_RX_ENABLE | IER_TX_ENABLE
            };
            ptr::write_volatile(reg_ptr(IER), ier);
        }
    }
}

pub static WRITER: SpinLock<Uart> = SpinLock::new("uart", Uart::new());
static PRINT_LOCK: SpinLock<()> = SpinLock::new("uart_print", ());

/// Bootstrap output is polled until global supervisor interrupts are live.
static ASYNC_OUTPUT: AtomicBool = AtomicBool::new(false);

pub fn init() {
    unsafe {
        ptr::write_volatile(reg_ptr(IER), 0x00);
        ptr::write_volatile(reg_ptr(IER), IER_RX_ENABLE);
    }
}

/// Switches output to the interrupt-driven transmit queue.
/// Call immediately before enabling global supervisor interrupts.
pub(crate) fn start_interrupts() {
    ASYNC_OUTPUT.store(true, Ordering::Release);
}

#[inline]
pub(crate) fn putchar(ch: u8) {
    let mut writer = WRITER.lock();
    if !ASYNC_OUTPUT.load(Ordering::Acquire) {
        writer.putc_raw(ch);
        return;
    }

    let next = Uart::tx_next(writer.tx_w);
    if next == writer.tx_r {
        // Preserve byte order without waiting for an interrupt while locked.
        let queued = writer.tx[writer.tx_r];
        writer.putc_raw(queued);
        writer.tx_r = Uart::tx_next(writer.tx_r);
    }
    let w = writer.tx_w;
    writer.tx[w] = ch;
    writer.tx_w = next;
    writer.transmit();
}

#[inline]
pub(crate) fn getchar() -> Option<u8> {
    unsafe {
        if (ptr::read_volatile(reg_ptr(LSR)) & LSR_RX_READY) != 0 {
            Some(ptr::read_volatile(reg_ptr(RHR)))
        } else {
            None
        }
    }
}

/// Drains receive and transmit UART interrupts.
pub(crate) fn handle_interrupt() {
    loop {
        let iir = unsafe { ptr::read_volatile(reg_ptr(IIR)) };
        if (iir & IIR_NO_PENDING) != 0 {
            break;
        }
        match iir & IIR_ID_MASK {
            IIR_TX_EMPTY => WRITER.lock().transmit(),
            _ => {
                while let Some(ch) = getchar() {
                    crate::drivers::console::intr(ch);
                }
            }
        }
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let _output = PRINT_LOCK.lock();
    crate::drivers::console::_print(args);
}

/// Prints formatted text to the UART console.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::drivers::uart::_print(format_args!($($arg)*));
    };
}

/// Prints formatted text to the UART console with an appended newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\n"));
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print!(concat!($fmt, "\n"), $($arg)*);
    };
}
