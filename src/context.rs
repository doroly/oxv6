//! RISC-V task context.
//!
//! The context contains the callee-saved registers required by the
//! RISC-V calling convention. It is used by the low-level context
//! switch routine to suspend and resume kernel tasks.

use core::arch::global_asm;

/// Saved CPU context for a RISC-V kernel task.
///
/// The layout of this structure must exactly match the layout used
/// by `context_switch` in RISC-V assembly.
///
/// Offset:
///
/// ```text
/// 0   ra
/// 8   sp
/// 16  s0
/// 24  s1
/// 32  s2
/// 40  s3
/// 48  s4
/// 56  s5
/// 64  s6
/// 72  s7
/// 80  s8
/// 88  s9
/// 96  s10
/// 104 s11
/// ```
///
/// The total size is 112 bytes on RV64.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Context {
    /// Return address register.
    pub(crate) ra: usize,

    /// Stack pointer register.
    pub(crate) sp: usize,

    /// Callee-saved register s0.
    pub(crate) s0: usize,

    /// Callee-saved register s1.
    pub(crate) s1: usize,

    /// Callee-saved register s2.
    pub(crate) s2: usize,

    /// Callee-saved register s3.
    pub(crate) s3: usize,

    /// Callee-saved register s4.
    pub(crate) s4: usize,

    /// Callee-saved register s5.
    pub(crate) s5: usize,

    /// Callee-saved register s6.
    pub(crate) s6: usize,

    /// Callee-saved register s7.
    pub(crate) s7: usize,

    /// Callee-saved register s8.
    pub(crate) s8: usize,

    /// Callee-saved register s9.
    pub(crate) s9: usize,

    /// Callee-saved register s10.
    pub(crate) s10: usize,

    /// Callee-saved register s11.
    pub(crate) s11: usize,
}

impl Context {
    /// Creates an empty context.
    pub(crate) const fn zero() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
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
        }
    }

    /// Creates the initial context of a new task.
    ///
    /// When this context is restored for the first time,
    /// `ret` will jump to `entry` and execution will continue
    /// from the task entry function.
    pub(crate) const fn new(entry: usize, stack_top: usize) -> Self {
        Self {
            ra: entry,
            sp: stack_top,

            s0: 0,
            s1: 0,
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
        }
    }
}

global_asm!(
    r#"
    .section .text                      # Place the following code in the .text section
    .globl context_switch               # Export the symbol so it can be called from Rust
    .type context_switch, @function     # Mark context_switch as a function

context_switch:
    # a0 = pointer to current Context
    # a1 = pointer to next Context

    # ---------- Save current task context ----------
    sd ra,   0(a0)
    sd sp,   8(a0)

    sd s0,  16(a0)
    sd s1,  24(a0)
    sd s2,  32(a0)
    sd s3,  40(a0)
    sd s4,  48(a0)
    sd s5,  56(a0)
    sd s6,  64(a0)
    sd s7,  72(a0)
    sd s8,  80(a0)
    sd s9,  88(a0)
    sd s10, 96(a0)
    sd s11,104(a0)

    # ---------- Restore next task context ----------
    ld ra,   0(a1)
    ld sp,   8(a1)

    ld s0,  16(a1)
    ld s1,  24(a1)
    ld s2,  32(a1)
    ld s3,  40(a1)
    ld s4,  48(a1)
    ld s5,  56(a1)
    ld s6,  64(a1)
    ld s7,  72(a1)
    ld s8,  80(a1)
    ld s9,  88(a1)
    ld s10, 96(a1)
    ld s11,104(a1)

    # Jump to the restored task (return address is now in ra)
    ret

    .size context_switch, .-context_switch
"#
);
