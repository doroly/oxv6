//! Supervisor-timer utilities used to schedule periodic interrupts.
//!
//! The kernel programs the next timer event by reading the platform clock and issuing the SBI
//! `set_timer` call. This allows the trap handler to drive round-robin preemption.

use core::arch::asm;

/// SBI legacy extension ID for `set_timer`.
const SBI_SET_TIMER: usize = 0;

/// Development timer interval in machine cycles.
pub(crate) const TIMER_INTERVAL: u64 = 5_000_000;

/// Reads the RISC‑V time CSR using the `rdtime` instruction and returns it.
#[inline]
pub(crate) fn read_time() -> u64 {
    let value;

    unsafe {
        asm!(
            "rdtime {}",
            out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Requests the next timer interrupt through the SBI interface.
///
/// `a0` = time value, `a7` = SBI_SET_TIMER, then executes `ecall`.
pub(crate) fn set_timer(time: u64) {
    unsafe {
        asm!(
            "ecall",
            in("a0") time as usize,
            in("a7") SBI_SET_TIMER,
            clobber_abi("C"),
        );
    }
}

/// Programs the next supervisor-timer interrupt based on the current time.
pub(crate) fn set_next_timer() {
    let now = read_time();
    set_timer(now + TIMER_INTERVAL);
}