//! Supervisor Binary Interface (SBI) calls.

const SBI_EXT_HSM: usize = 0x48534D; // HSM Extension ID ("HSM")
const SBI_HSM_HART_START: usize = 0;  // Function ID for hart_start

/// Asynchronously starts execution on a secondary hart via OpenSBI.
///
/// # Arguments
///
/// * `hartid` - Target hart ID to start.
/// * `start_addr` - Target execution start physical address (typically `_start`).
/// * `opaque` - Value passed to target hart in `a1` register.
pub(crate) fn sbi_hart_start(hartid: usize, start_addr: usize, opaque: usize) {
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") SBI_EXT_HSM,
        in("a6") SBI_HSM_HART_START,
        in("a0") hartid,
        in("a1") start_addr,
        in("a2") opaque,
        options(nomem, nostack)
        );
    }
}
