//! RISC-V Supervisor-Mode (S-mode) Trap Handling Subsystem.
//!
//! This module provides:
//! - TrapFrame definition.
//! - Low-level trap entry/return code.
//! - Supervisor timer interrupt handling.
//! - Trap-driven preemptive task switching.
//!
//! The important design principle is:
//!
//!     TrapFrame = CPU execution context of a preempted task.
//!
//! When a timer interrupt occurs, the complete register state of the
//! current task is saved into its TrapFrame. The scheduler then selects
//! another task and returns the address of that task's TrapFrame.
//! `trap_entry` restores the selected TrapFrame and executes `sret`.

use crate::arch::riscv64::{csr, timer};
use crate::uart::{print_hex, print_str};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Register context saved when entering a Supervisor-mode trap.
///
/// This structure must exactly match the layout used by `trap_entry`.
///
/// RISC-V has 32 integer registers (`x0`-`x31`). `x0` (`zero`) is
/// hardwired to zero and therefore does not need to be saved.
///
/// The remaining 31 registers are stored here.
#[repr(C)]
pub(crate) struct TrapFrame {
    // x1  - return address
    pub(crate) ra: usize,

    // x2  - stack pointer
    pub(crate) sp: usize,

    // x3  - global pointer
    pub(crate) gp: usize,

    // x4  - thread pointer
    pub(crate) tp: usize,

    // x5-x7  - temporary registers
    pub(crate) t0: usize,   // x5
    pub(crate) t1: usize,   // x6
    pub(crate) t2: usize,   // x7

    // x8-x9  - saved registers (callee-saved)
    pub(crate) s0: usize,   // x8 / fp
    pub(crate) s1: usize,   // x9

    // x10-x17  - argument registers
    pub(crate) a0: usize,   // x10
    pub(crate) a1: usize,   // x11
    pub(crate) a2: usize,   // x12
    pub(crate) a3: usize,   // x13
    pub(crate) a4: usize,   // x14
    pub(crate) a5: usize,   // x15
    pub(crate) a6: usize,   // x16
    pub(crate) a7: usize,   // x17

    // x18-x27  - saved registers (callee-saved)
    pub(crate) s2: usize,   // x18
    pub(crate) s3: usize,   // x19
    pub(crate) s4: usize,   // x20
    pub(crate) s5: usize,   // x21
    pub(crate) s6: usize,   // x22
    pub(crate) s7: usize,   // x23
    pub(crate) s8: usize,   // x24
    pub(crate) s9: usize,   // x25
    pub(crate) s10: usize,  // x26
    pub(crate) s11: usize,  // x27

    // x28-x31  - temporary registers
    pub(crate) t3: usize,   // x28
    pub(crate) t4: usize,   // x29
    pub(crate) t5: usize,   // x30
    pub(crate) t6: usize,   // x31

    // Control/status registers restored before `sret`.
    pub(crate) sepc: usize,     // supervisor exception PC
    pub(crate) sstatus: usize,  // supervisor status register
}

/// Size of a TrapFrame in bytes.
///
/// There are 31 saved integer registers and each register is 8 bytes on RV64:
///
///     31 * 8 = 248 bytes
///
/// Plus two control/status registers:
///
///     sepc + sstatus = 16 bytes
///
/// Total payload is 264 bytes; we reserve 272 bytes to keep 16-byte alignment.
///
/// We reserve 272 bytes on the stack so that the frame remains
/// 16-byte aligned.
pub(crate) const TRAP_FRAME_SIZE: usize = 272;

/// Debug-only timer tick counter used to confirm that preemption is timer-driven.
static TIMER_TICK_COUNT: AtomicUsize = AtomicUsize::new(0);

impl TrapFrame {
    /// Builds the initial frame for a kernel task that starts at `entry`.
    pub(crate) const fn for_kernel_task(
        entry: usize,
        stack_top: usize,
        gp: usize,
        tp: usize,
        sstatus: usize,
    ) -> Self {
        Self {
            ra: 0,
            sp: stack_top,
            gp,
            tp,
            t0: 0,
            t1: 0,
            t2: 0,
            s0: 0,
            s1: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
            t3: 0,
            t4: 0,
            t5: 0,
            t6: 0,
            sepc: entry,
            sstatus,
        }
    }
}

core::arch::global_asm!(
   r#"
    .section .text.trap

    .globl trap_entry
    .type trap_entry, @function

# ============================================================================
# Supervisor Trap Entry Point
# ============================================================================
trap_entry:
    # Allocate 272 bytes on stack for TrapFrame
    addi sp, sp, -272

    # --- Save General Purpose Registers ---
    sd ra,   0(sp)

    # Calculate original stack pointer (sp + 272) before trap entry
    addi t0, sp, 272
    sd t0,   8(sp)

    sd gp,  16(sp)
    sd tp,  24(sp)
    sd t0,  32(sp)
    sd t1,  40(sp)
    sd t2,  48(sp)
    sd s0,  56(sp)
    sd s1,  64(sp)
    sd a0,  72(sp)
    sd a1,  80(sp)
    sd a2,  88(sp)
    sd a3,  96(sp)
    sd a4, 104(sp)
    sd a5, 112(sp)
    sd a6, 120(sp)
    sd a7, 128(sp)
    sd s2, 136(sp)
    sd s3, 144(sp)
    sd s4, 152(sp)
    sd s5, 160(sp)
    sd s6, 168(sp)
    sd s7, 176(sp)
    sd s8, 184(sp)
    sd s9, 192(sp)
    sd s10, 200(sp)
    sd s11, 208(sp)
    sd t3, 216(sp)
    sd t4, 224(sp)
    sd t5, 232(sp)
    sd t6, 240(sp)

    # --- Save Control and Status Registers (CSRs) ---
    csrr t0, sepc
    sd t0, 248(sp)
    csrr t0, sstatus
    sd t0, 256(sp)

    # --- Transfer Control to Rust Handler ---
    mv a0, sp           # Pass TrapFrame pointer as argument
    call rust_trap_handler

    j trap_return

# ============================================================================
# Supervisor Trap Return Point
# ============================================================================
    .globl trap_return
    .type trap_return, @function
trap_return:
    # Set stack pointer to target TrapFrame (returned in a0)
    mv sp, a0

    # --- Restore Control and Status Registers (CSRs) ---
    ld t1, 248(sp)
    csrw sepc, t1
    ld t1, 256(sp)
    csrw sstatus, t1

    # --- Restore General Purpose Registers ---
    ld ra,   0(sp)
    ld t0,   8(sp)      # Load original sp temporarily into t0
    ld gp,  16(sp)
    ld tp,  24(sp)
    ld t1,  40(sp)
    ld t2,  48(sp)
    ld s0,  56(sp)
    ld s1,  64(sp)
    ld a0,  72(sp)
    ld a1,  80(sp)
    ld a2,  88(sp)
    ld a3,  96(sp)
    ld a4, 104(sp)
    ld a5, 112(sp)
    ld a6, 120(sp)
    ld a7, 128(sp)
    ld s2, 136(sp)
    ld s3, 144(sp)
    ld s4, 152(sp)
    ld s5, 160(sp)
    ld s6, 168(sp)
    ld s7, 176(sp)
    ld s8, 184(sp)
    ld s9, 192(sp)
    ld s10, 200(sp)
    ld s11, 208(sp)
    ld t3, 216(sp)
    ld t4, 224(sp)
    ld t5, 232(sp)
    ld t6, 240(sp)

    # --- Restore Final Registers ---
    ld t0,  32(sp)      # Restore actual saved t0 value
    ld sp,   8(sp)      # Restore original task stack pointer

    # Return from Supervisor Trap
    sret
"#
);

unsafe extern "C" {
    fn trap_entry();
}

/// Common Supervisor-mode trap handler.
///
/// Returns the TrapFrame that should be restored by `trap_entry`.
///
/// For a normal trap:
///
///     current_frame -> current_frame
///
/// For a timer-triggered preemption:
///
///     current_frame -> next_task_frame
///
/// This design allows the trap handler to perform task switching
/// without attempting to switch the kernel stack while the assembly
/// trap-return sequence is still active.
#[unsafe(no_mangle)]
pub extern "C" fn rust_trap_handler(frame: &mut TrapFrame) -> *mut TrapFrame {
    let cause = csr::read_scause();

    // The highest bit of scause indicates whether the cause is
    // an interrupt or an exception.
    let is_interrupt = (cause >> (usize::BITS - 1)) != 0;

    // Remove the interrupt bit and obtain the exception code.
    let code = cause & !(1usize << (usize::BITS - 1));

    // ------------------------------------------------------------
    // Supervisor Timer Interrupt
    // ------------------------------------------------------------

    if is_interrupt && code == 5 {
        // Program the next timer event first.
        //
        // Otherwise, the current interrupt would not be followed
        // by another timer interrupt.
        timer::set_next_timer();

        let ticks = TIMER_TICK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if ticks % 10 == 0 {
            print_str("[timer] tick=");
            print_hex(ticks);
            print_str("\n");
        }

        // Ask the scheduler whether another task should run.
        //
        // `timer_tick()` returns the TrapFrame of the task that
        // should continue execution.
        return crate::task::timer_tick(frame);
    }

    // ------------------------------------------------------------
    // Unhandled trap
    // ------------------------------------------------------------

    print_str("\n========== UNHANDLED TRAP ==========\n");
    print_str("scause = ");
    print_hex(cause);
    print_str("\n");
    print_str("sepc   = ");
    print_hex(csr::read_sepc());
    print_str("\n");
    print_str("stval  = ");
    print_hex(csr::read_stval());
    print_str("\n");
    print_str("====================================\n");

    loop {
        core::hint::spin_loop();
    }
}

/// Initializes Supervisor-mode trap handling during early boot.
///
/// The function installs the kernel trap vector and enables the timer and global interrupt bits
/// required for timer-preemptive scheduling and trap delivery.
pub(crate) fn init() {
    csr::write_stvec(trap_entry as *const () as usize);
    csr::enable_timer_interrupt();
    csr::enable_interrupts();
}
