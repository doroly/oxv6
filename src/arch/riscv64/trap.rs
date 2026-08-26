//! RISC-V Supervisor-Mode (S-mode) Trap Handling Subsystem.
//!
//! This module provides:
//! - `TrapFrame` structure matching the low-level context layout.
//! - Low-level assembly entry and return routines (`trap_entry` / `trap_return`).
//! - High-level Rust trap handling logic and interrupt dispatching.
//!
//! # Core Design Principle
//!
//! A `TrapFrame` represents the CPU execution context of a preempted or interrupted task.
//! When a trap occurs, all 31 general-purpose registers and key CSRs are saved into a
//! `TrapFrame` allocated on the stack. The trap handler processes the event (e.g., timer
//! preemption) and returns a pointer to the `TrapFrame` that should be restored next.

use crate::arch::riscv64::{csr, timer};
use crate::drivers::plic;
use crate::println;
use crate::task::scheduler::timer_tick;
use core::arch::asm;

/// Register context saved when entering a Supervisor-mode trap.
///
/// This structure must exactly match the stack offset layout used by `trap_entry` and `trap_return`.
/// RISC-V has 32 integer registers (`x0`-`x31`), where `x0` (`zero`) is hardwired to zero
/// and does not need to be saved.
#[repr(C)]
pub(crate) struct TrapFrame {
    // x1  - Return address
    pub(crate) ra: usize,

    // x2  - Stack pointer
    pub(crate) sp: usize,

    // x3  - Global pointer
    pub(crate) gp: usize,

    // x4  - Thread pointer
    pub(crate) tp: usize,

    // x5-x7  - Temporary registers
    pub(crate) t0: usize, // x5
    pub(crate) t1: usize, // x6
    pub(crate) t2: usize, // x7

    // x8-x9  - Saved registers (callee-saved / frame pointer)
    pub(crate) s0: usize, // x8 / fp
    pub(crate) s1: usize, // x9

    // x10-x17 - Function argument / return value registers
    pub(crate) a0: usize, // x10
    pub(crate) a1: usize, // x11
    pub(crate) a2: usize, // x12
    pub(crate) a3: usize, // x13
    pub(crate) a4: usize, // x14
    pub(crate) a5: usize, // x15
    pub(crate) a6: usize, // x16
    pub(crate) a7: usize, // x17

    // x18-x27 - Saved registers (callee-saved)
    pub(crate) s2: usize,  // x18
    pub(crate) s3: usize,  // x19
    pub(crate) s4: usize,  // x20
    pub(crate) s5: usize,  // x21
    pub(crate) s6: usize,  // x22
    pub(crate) s7: usize,  // x23
    pub(crate) s8: usize,  // x24
    pub(crate) s9: usize,  // x25
    pub(crate) s10: usize, // x26
    pub(crate) s11: usize, // x27

    // x28-x31 - Temporary registers
    pub(crate) t3: usize, // x28
    pub(crate) t4: usize, // x29
    pub(crate) t5: usize, // x30
    pub(crate) t6: usize, // x31

    // Control and Status Registers (CSRs)
    pub(crate) sepc: usize,    // Supervisor exception program counter
    pub(crate) sstatus: usize, // Supervisor status register
}

/// Size of a `TrapFrame` in bytes (272 bytes).
///
/// Payload: 31 integer registers (248B) + 2 CSRs (16B) = 264 bytes.
/// Reserved 272 bytes to maintain 16-byte stack alignment required by RISC-V ABI.
pub(crate) const TRAP_FRAME_SIZE: usize = 272;

impl TrapFrame {
    /// Constructs an initial `TrapFrame` for a kernel task starting at `entry`.
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

/// Initializes Supervisor-mode trap handling during system boot.
///
/// Sets the Supervisor trap vector register (`stvec`) and enables
/// timer and external interrupts in the Supervisor Interrupt Enable (`sie`) CSR.
pub(crate) fn init() {
    let vector = (trap_entry as *const () as usize) & !0b11;
    csr::write_stvec(vector);
    csr::enable_timer_interrupt();
    csr::enable_external_interrupt();
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
    # Allocate 272 bytes on the stack for TrapFrame
    addi sp, sp, -272

    # --- Save General Purpose Registers ---
    # Save original t0 FIRST before using it as a temporary scratch register
    sd t0, 32(sp)

    sd ra,   0(sp)

    # Calculate and store original stack pointer (sp + 272)
    addi t0, sp, 272
    sd t0,   8(sp)

    sd gp,  16(sp)
    sd tp,  24(sp)
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

    # --- Transfer Control to High-Level Rust Handler ---
    mv a0, sp           # Pass pointer to current TrapFrame as first argument
    call rust_trap_handler

    j trap_return

# ============================================================================
# Supervisor Trap Return Point
# ============================================================================
    .globl trap_return
    .type trap_return, @function
trap_return:
    # Set stack pointer to target TrapFrame pointer returned in a0
    mv sp, a0

    # --- Restore Control and Status Registers (CSRs) ---
    ld t0, 248(sp)
    csrw sepc, t0
    ld t0, 256(sp)
    csrw sstatus, t0

    # --- Restore General Purpose Registers ---
    ld ra,   0(sp)
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
    ld t0, 32(sp)       # Restore original t0 value
    ld sp,  8(sp)       # Restore task's original stack pointer

    # Return to Supervisor mode
    sret
"#
);

unsafe extern "C" {
    fn trap_entry();
}

/// Central C-ABI entry point for Supervisor-mode trap handling.
///
/// Accepts a mutable reference to the interrupted context (`TrapFrame`) and returns
/// a pointer to the `TrapFrame` to be restored upon returning via `sret`.
#[unsafe(no_mangle)]
pub extern "C" fn rust_trap_handler(frame: &mut TrapFrame) -> *mut TrapFrame {
    let cause = csr::read_scause();
    let interrupt_bit = 1usize << (usize::BITS - 1);
    let is_interrupt = (cause & interrupt_bit) != 0;
    let code = cause & !interrupt_bit;

    if is_interrupt {
        match code {
            // Supervisor software interrupt.
            1 => {
                println!("Supervisor software interrupt");
            }
            // Supervisor timer interrupt.
            5 => {
                timer::set_next_timer();
                return timer_tick(frame);
            }
            // Supervisor external interrupt.
            9 => {
                handle_external_interrupt();
            }
            _ => {
                println!("Unknown interrupt: {:#x}", code);
            }
        }

        return frame as *mut TrapFrame;
    }

    // Unhandled exception / trap.
    println!("\n========== UNHANDLED TRAP ==========");
    println!("scause = {:#018x}", cause);
    println!("sepc   = {:#018x}", csr::read_sepc());
    println!("stval  = {:#018x}", csr::read_stval());
    println!("====================================");

    loop {
        core::hint::spin_loop();
    }
}

/// Reads the thread pointer (`tp`) register to obtain the current Hart ID.
#[inline]
fn current_hartid() -> usize {
    let hartid: usize;
    unsafe {
        asm!(
        "mv {}, tp",
        out(reg) hartid,
        options(nomem, nostack, preserves_flags)
        );
    }
    hartid
}

/// Handles external interrupts delivered by the PLIC for the active hart.
pub(crate) fn handle_external_interrupt() {
    // 1. Obtain the executing hart ID to access hart-specific PLIC contexts
    let hartid = current_hartid();

    // 2. Claim the highest priority pending interrupt for this hart
    let irq = plic::claim(hartid);

    match irq {
        plic::UART0_IRQ => {
            crate::drivers::uart::handle_interrupt();
        }
        plic::VIRTIO0_IRQ => {
            println!("[Hart {}] VirtIO interrupt received", hartid);
        }
        0 => {
            // Spurious or no pending interrupt for this hart
        }
        _ => {
            println!("[Hart {}] Unknown external IRQ: {:#x}", hartid, irq);
        }
    }

    // 3. Complete the interrupt to allow future IRQs on this hart
    if irq != 0 {
        plic::complete(hartid, irq);
    }
}
