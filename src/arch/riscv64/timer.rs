//! Supervisor-timer utilities used to schedule periodic interrupts.
//!
//! The kernel programs the next timer event by reading the platform hardware clock (`time` CSR)
//! and issuing an SBI `set_timer` system call (`ecall`). This enables the trap handler
//! to drive periodic round-robin preemption.

use core::arch::asm;

/// Legacy SBI Extension ID for `set_timer` (`EID = 0`).
const SBI_SET_TIMER: usize = 0;

/// Default timer interrupt interval in machine hardware cycles.
pub(crate) const TIMER_INTERVAL: u64 = 5_000_000;

/// Reads the current 64-bit RISC-V hardware time counter using the `rdtime` instruction.
#[inline]
pub(crate) fn read_time() -> u64 {
    let value: u64;
    unsafe {
        asm!(
        "rdtime {}",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
    value
}

/// Requests the next timer interrupt from the SEE (Supervisor Execution Environment / OpenSBI).
///
/// Passes `a7 = SBI_SET_TIMER` and `a0 = stime_value` via an `ecall`.
///
/// # Arguments
///
/// * `stime_value` - Absolute hardware timestamp at which the next timer interrupt should trigger.
#[inline]
pub(crate) fn set_timer(stime_value: u64) {
    unsafe {
        asm!(
            "ecall",
            in("a7") SBI_SET_TIMER,
            in("a0") stime_value as usize,
            clobber_abi("C"),
        );
    }
}

/// Programs the next supervisor-timer interrupt based on the current hardware clock.
#[inline]
pub(crate) fn set_next_timer() {
    let now = read_time();
    set_timer(now + TIMER_INTERVAL);
}

/// Initializes the supervisor timer so that periodic interrupts begin.
///
/// This should be called once during kernel startup (e.g., before enabling interrupts).
#[inline]
pub(crate) fn init() {
    // Program the first timer event.
    set_next_timer();
}
