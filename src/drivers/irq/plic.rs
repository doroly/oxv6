//! RISC-V Platform-Level Interrupt Controller (PLIC).
//!
//! Handles external hardware interrupt routing, priority thresholds,
//! claim, and completion for the QEMU `virt` platform in multi-hart environments.

use core::mem::size_of;
use core::ptr;

/// QEMU virt PLIC base physical address.
const PLIC_BASE: usize = 0x0c00_0000;

/// Base address for per-source priority registers.
const PLIC_PRIORITY: usize = PLIC_BASE;

/// UART0 interrupt source number.
pub(crate) const UART0_IRQ: usize = 10;

/// VirtIO disk interrupt source number.
pub(crate) const VIRTIO0_IRQ: usize = 1;

/// Computes the priority register pointer for a given IRQ source.
#[inline]
const fn source_priority_ptr(irq: usize) -> *mut u32 {
    (PLIC_PRIORITY + irq * size_of::<u32>()) as *mut u32
}

/// Computes the Enable register address for a given hart's S-mode context.
/// Context for Hart N in S-mode is `2 * N + 1`.
#[inline]
const fn senable_ptr(hartid: usize) -> *mut u32 {
    let context = 2 * hartid + 1;
    (PLIC_BASE + 0x2000 + context * 0x80) as *mut u32
}

/// Computes the Threshold register address for a given hart's S-mode context.
#[inline]
const fn sthreshold_ptr(hartid: usize) -> *mut u32 {
    let context = 2 * hartid + 1;
    (PLIC_BASE + 0x200000 + context * 0x1000) as *mut u32
}

/// Computes the Claim/Complete register address for a given hart's S-mode context.
#[inline]
const fn sclaim_ptr(hartid: usize) -> *mut u32 {
    let context = 2 * hartid + 1;
    (PLIC_BASE + 0x200000 + context * 0x1000 + 4) as *mut u32
}

/// Initializes global PLIC hardware priorities (called once by Hart 0).
pub(crate) fn init() {
    unsafe {
        // Set priority > 0 for active IRQs
        ptr::write_volatile(source_priority_ptr(UART0_IRQ), 1);
        ptr::write_volatile(source_priority_ptr(VIRTIO0_IRQ), 1);
    }
}

/// Enables interrupts and sets threshold for the calling hart's S-mode context.
pub(crate) fn init_hart(hartid: usize) {
    let enable_mask = (1u32 << UART0_IRQ) | (1u32 << VIRTIO0_IRQ);

    unsafe {
        // Enable UART0 and VirtIO0 interrupts for this hart's S-mode
        ptr::write_volatile(senable_ptr(hartid), enable_mask);

        // Set priority threshold to 0 to accept all priority > 0 interrupts
        ptr::write_volatile(sthreshold_ptr(hartid), 0);
    }
}

/// Claims the highest priority pending interrupt for the specified hart.
#[inline]
pub(crate) fn claim(hartid: usize) -> usize {
    unsafe { ptr::read_volatile(sclaim_ptr(hartid) as *const u32) as usize }
}

/// Signals completion of handling the specified interrupt source for the given hart.
#[inline]
pub(crate) fn complete(hartid: usize, irq: usize) {
    unsafe {
        ptr::write_volatile(sclaim_ptr(hartid), irq as u32);
    }
}
