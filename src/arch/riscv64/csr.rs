#![allow(dead_code)]

//! RISC-V Supervisor-mode CSR access helpers.
//!
//! This module wraps the low-level `csrr` / `csrw` instructions used by the kernel to configure
//! traps, interrupts, and the current execution state in S-mode.

use core::arch::asm;

/// `sstatus.SIE`.
pub(crate) const SSTATUS_SIE: usize = 1 << 1;

/// `sstatus.SPIE`.
pub(crate) const SSTATUS_SPIE: usize = 1 << 5;

/// Supervisor timer interrupt enable a bit in `sie`.
pub(crate) const SIE_STIE: usize = 1 << 5;

/// `sstatus.SPP`.
///
/// 0 = User
/// 1 = Supervisor
pub(crate) const SSTATUS_SPP: usize = 1 << 8;

/// Read `sstatus`.
#[inline]
pub(crate) fn read_sstatus() -> usize {
    let value;

    unsafe {
        asm!(
        "csrr {}, sstatus",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Enable Supervisor interrupts globally.
#[inline]
pub(crate) fn enable_interrupts() {
    unsafe {
        asm!(
        "csrs sstatus, {}",
        in(reg) SSTATUS_SIE,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Disable Supervisor interrupts globally.
#[inline]
pub(crate) fn disable_interrupts() {
    unsafe {
        asm!(
        "csrc sstatus, {}",
        in(reg) SSTATUS_SIE,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Enable Supervisor timer interrupts.
#[inline]
pub(crate) fn enable_timer_interrupt() {
    unsafe {
        asm!(
        "csrs sie, {}",
        in(reg) SIE_STIE,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Disable Supervisor timer interrupts.
#[inline]
pub(crate) fn disable_timer_interrupt() {
    unsafe {
        asm!(
        "csrc sie, {}",
        in(reg) SIE_STIE,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Read `scause`.
#[inline]
pub(crate) fn read_scause() -> usize {
    let value;

    unsafe {
        asm!(
        "csrr {}, scause",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Read `sepc`.
#[inline]
pub(crate) fn read_sepc() -> usize {
    let value;

    unsafe {
        asm!(
        "csrr {}, sepc",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Write `sepc`.
#[inline]
pub(crate) fn write_sepc(value: usize) {
    unsafe {
        asm!(
        "csrw sepc, {}",
        in(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Read `stval`.
#[inline]
pub(crate) fn read_stval() -> usize {
    let value;

    unsafe {
        asm!(
        "csrr {}, stval",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Write Supervisor trap vector.
#[inline]
pub(crate) fn write_stvec(value: usize) {
    unsafe {
        asm!(
        "csrw stvec, {}",
        in(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Write `sstatus`.
#[inline]
pub(crate) fn write_sstatus(value: usize) {
    unsafe {
        asm!(
        "csrw sstatus, {}",
        in(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Set bits in `sstatus`.
#[inline]
pub(crate) fn set_sstatus(bits: usize) {
    unsafe {
        asm!(
        "csrs sstatus, {}",
        in(reg) bits,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Clear bits in `sstatus`.
#[inline]
pub(crate) fn clear_sstatus(bits: usize) {
    unsafe {
        asm!(
        "csrc sstatus, {}",
        in(reg) bits,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Read `sie`.
#[inline]
pub(crate) fn read_sie() -> usize {
    let value;

    unsafe {
        asm!(
        "csrr {}, sie",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Write `sie`.
#[inline]
pub(crate) fn write_sie(value: usize) {
    unsafe {
        asm!(
        "csrw sie, {}",
        in(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Set bits in `sie`.
#[inline]
pub(crate) fn set_sie(bits: usize) {
    unsafe {
        asm!(
        "csrs sie, {}",
        in(reg) bits,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Clear bits in `sie`.
#[inline]
pub(crate) fn clear_sie(bits: usize) {
    unsafe {
        asm!(
        "csrc sie, {}",
        in(reg) bits,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Read `stvec`.
#[inline]
pub(crate) fn read_stvec() -> usize {
    let value;

    unsafe {
        asm!(
        "csrr {}, stvec",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Read `satp`.
#[inline]
pub(crate) fn read_satp() -> usize {
    let value;

    unsafe {
        asm!(
        "csrr {}, satp",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Write `satp`.
#[inline]
pub(crate) unsafe fn write_satp(value: usize) {
    unsafe {
        asm!(
        "csrw satp, {}",
        in(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
}
