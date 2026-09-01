#![allow(dead_code)]

//! RISC-V Supervisor-mode CSR access helpers.
//!
//! This module wraps the low-level `csrr` / `csrw` instructions used by the kernel to configure
//! traps, interrupts, and the current execution state in S-mode.

use crate::arch::riscv64::boot::MAX_HARTS;
use core::arch::asm;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// `sstatus.SIE`.
pub(crate) const SSTATUS_SIE: usize = 1 << 1;

/// `sstatus.SPIE`.
pub(crate) const SSTATUS_SPIE: usize = 1 << 5;

/// Constant for Supervisor External Interrupt Enable bit (SEIE, bit 9) in `sie`.
pub(crate) const SIE_SEIE: usize = 1 << 9;

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

/// Enable Supervisor external interrupts.
#[inline]
pub(crate) fn enable_external_interrupt() {
    unsafe {
        asm!(
        "csrs sie, {}",
        in(reg) SIE_SEIE,
        options(nomem, nostack, preserves_flags),
        );
    }
}

/// Disable Supervisor external interrupts.
#[inline]
pub(crate) fn disable_external_interrupt() {
    unsafe {
        asm!(
        "csrc sie, {}",
        in(reg) SIE_SEIE,
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

/// Flush all local TLB entries after changing address translation state.
#[inline]
pub(crate) fn sfence_vma() {
    unsafe {
        asm!("sfence.vma zero, zero", options(nostack, preserves_flags));
    }
}

/// Per-hart nesting counter for interrupt disable depth.
/// Tracks how many times interrupts have been disabled on each hart.
static NOFF: [AtomicUsize; MAX_HARTS] = [const { AtomicUsize::new(0) }; MAX_HARTS];

/// Per-hart flag indicating whether interrupts were enabled before being disabled.
/// Stores the interrupt state prior to the first `push_off` call.
static INTENA: [AtomicBool; MAX_HARTS] = [const { AtomicBool::new(false) }; MAX_HARTS];

/// Disables local supervisor interrupts and increments nesting count.
///
/// This function is used to implement interrupt-safe critical sections.
/// It saves the current interrupt state on the first call and disables interrupts.
/// Subsequent calls increment a nesting counter without changing interrupt state.
/// Must be paired with `pop_off` to restore the original interrupt state.
pub(crate) fn push_off() {
    let old_sstatus = read_sstatus();
    let old_sie = (old_sstatus & SSTATUS_SIE) != 0;
    disable_interrupts();

    let hartid = read_hartid();
    if hartid >= MAX_HARTS {
        panic!("push_off: invalid hartid {}", hartid);
    }

    if NOFF[hartid].load(Ordering::Relaxed) == 0 {
        INTENA[hartid].store(old_sie, Ordering::Relaxed);
    }
    NOFF[hartid].fetch_add(1, Ordering::Relaxed);
}

/// Restores previous interrupt state when nested locks are fully released.
///
/// Decrements the nesting counter and re-enables interrupts if this is the
/// final `pop_off` call (nesting counter reaches zero) and interrupts were
/// originally enabled before the first `push_off`.
/// Must be paired with a corresponding `push_off` call.
pub(crate) fn pop_off() {
    let hartid = read_hartid();
    if hartid >= MAX_HARTS {
        panic!("pop_off: invalid hartid {}", hartid);
    }
    if (read_sstatus() & SSTATUS_SIE) != 0 {
        panic!("pop_off: interrupts enabled while holding spinlock");
    }

    let noff = NOFF[hartid].load(Ordering::Relaxed);
    if noff == 0 {
        panic!("pop_off: unbalanced unlock");
    }
    NOFF[hartid].store(noff - 1, Ordering::Relaxed);

    if noff == 1 && INTENA[hartid].load(Ordering::Relaxed) {
        enable_interrupts();
    }
}

/// Reads the current Hart ID (`mhartid` or passed via `tp`/SBI).
#[inline]
pub(crate) fn read_hartid() -> usize {
    let id: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) id, options(nomem, nostack));
    }
    id
}
