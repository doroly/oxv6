//! RISC-V Platform-Level Interrupt Controller (PLIC).
//!
//! Handles external hardware interrupt routing, priority thresholds,
//! claim, and completion for the QEMU `virt` platform.

use core::ptr;

/// QEMU virt PLIC base physical address.
const PLIC_BASE: usize = 0x0c00_0000;

/// Base address for per-source priority registers.
const PLIC_PRIORITY: usize = PLIC_BASE;

/// UART0 interrupt source number.
pub(crate) const UART0_IRQ: usize = 10;

/// VirtIO disk interrupt source number.
pub(crate) const VIRTIO0_IRQ: usize = 1;

/// Enable register base for Hart 0 Supervisor mode (Context 1: `0x2000 + 0x80`).
const PLIC_SENABLE: usize = PLIC_BASE + 0x2080;

/// Priority threshold register for Hart 0 Supervisor mode (Context 1: `0x200000 + 0x1000`).
const PLIC_STHRESHOLD: usize = PLIC_BASE + 0x201000;

/// Interrupt claim/complete register for Hart 0 Supervisor mode.
const PLIC_SCLAIM: usize = PLIC_BASE + 0x201004;

/// Computes the pointer to the priority register for a given IRQ source.
#[inline]
const fn source_priority_ptr(irq: usize) -> *mut u32 {
    (PLIC_PRIORITY + irq * size_of::<u32>()) as *mut u32
}

/// Initializes the PLIC hardware.
///
/// Configures interrupt priorities, enables UART0 and VirtIO0 interrupts,
/// and sets the Supervisor threshold to accept all non-zero priority interrupts.
pub(crate) fn init() {
    let enable_mask = (1u32 << UART0_IRQ) | (1u32 << VIRTIO0_IRQ);

    unsafe {
        // Priorities must be non-zero (greater than threshold = 0) to trigger interrupts.
        ptr::write_volatile(source_priority_ptr(UART0_IRQ), 1);
        ptr::write_volatile(source_priority_ptr(VIRTIO0_IRQ), 1);

        // Enable interrupt sources for S-mode.
        ptr::write_volatile(PLIC_SENABLE as *mut u32, enable_mask);

        // Set threshold to 0 to accept all priority > 0 interrupts.
        ptr::write_volatile(PLIC_STHRESHOLD as *mut u32, 0);
    }
}

/// Claims the highest priority pending interrupt.
///
/// Returns:
/// - `0` if no interrupt is pending.
/// - Non-zero IRQ source number otherwise.
#[inline]
pub(crate) fn claim() -> usize {
    unsafe { ptr::read_volatile(PLIC_SCLAIM as *const u32) as usize }
}

/// Signals completion of handling the specified interrupt source.
///
/// Must be called with the exact `irq` ID returned by `claim()`.
#[inline]
pub(crate) fn complete(irq: usize) {
    unsafe {
        ptr::write_volatile(PLIC_SCLAIM as *mut u32, irq as u32);
    }
}