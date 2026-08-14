#![allow(dead_code)]

//! Saved task context for RISC-V kernel threads.
//!
//! The kernel stores the callee-saved registers required by the RISC-V ABI in this structure
//! so it can suspend and resume tasks without losing their execution state.

/// Saved CPU context for a RISC-V kernel task.
///
/// The layout of this structure must exactly match the layout used
/// by `context_switch` in RISC-V assembly.
///
/// # Layout (112 bytes on RV64)
/// - `0x00`: `ra`  (Return address)
/// - `0x08`: `sp`  (Stack pointer)
/// - `0x10` - `0x68`: `s0`-`s11` (Callee-saved registers)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Context {
    /// Return address register.
    pub(crate) ra: usize,

    /// Stack pointer register.
    pub(crate) sp: usize,

    /// Callee-saved registers (s0 / fp, s1-s11).
    pub(crate) s0: usize,
    pub(crate) s1: usize,
    pub(crate) s2: usize,
    pub(crate) s3: usize,
    pub(crate) s4: usize,
    pub(crate) s5: usize,
    pub(crate) s6: usize,
    pub(crate) s7: usize,
    pub(crate) s8: usize,
    pub(crate) s9: usize,
    pub(crate) s10: usize,
    pub(crate) s11: usize,
}

impl Context {
    /// Creates an empty zero-initialized context.
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

    /// Creates the initial execution context for a new kernel task.
    ///
    /// When this context is switched into for the first time, `ret` will
    /// jump to `entry` with the stack pointer set to `stack_top`.
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

core::arch::global_asm!(
    r#"
    .section .text
    .globl context_switch
    .type context_switch, @function

# ============================================================================
# Low-level RISC-V Context Switch Entry Point
#
# Arguments:
#   a0 = *mut Context  (Pointer to current task's context buffer)
#   a1 = *const Context (Pointer to next task's context buffer)
# ============================================================================
context_switch:
    # --- Save Current Task Context ---
    sd ra,    0(a0)
    sd sp,    8(a0)
    sd s0,   16(a0)
    sd s1,   24(a0)
    sd s2,   32(a0)
    sd s3,   40(a0)
    sd s4,   48(a0)
    sd s5,   56(a0)
    sd s6,   64(a0)
    sd s7,   72(a0)
    sd s8,   80(a0)
    sd s9,   88(a0)
    sd s10,  96(a0)
    sd s11, 104(a0)

    # --- Restore Target Task Context ---
    ld ra,    0(a1)
    ld sp,    8(a1)
    ld s0,   16(a1)
    ld s1,   24(a1)
    ld s2,   32(a1)
    ld s3,   40(a1)
    ld s4,   48(a1)
    ld s5,   56(a1)
    ld s6,   64(a1)
    ld s7,   72(a1)
    ld s8,   80(a1)
    ld s9,   88(a1)
    ld s10,  96(a1)
    ld s11, 104(a1)

    # Jump to the target task's saved return address (ra)
    ret

    .size context_switch, . - context_switch
"#
);

unsafe extern "C" {
    /// Low-level assembly routine for switching between two task contexts.
    fn context_switch(current: *mut Context, next: *const Context);
}

/// Performs a kernel context switch from `current` to `next`.
///
/// # Safety
///
/// - `current` must point to a valid, writable `Context` buffer to hold the active state.
/// - `next` must point to a valid, properly initialized `Context` to restore.
/// - Both stack pointers referenced in the contexts must be valid and 16-byte aligned.
#[inline]
pub(crate) fn switch(current: *mut Context, next: *const Context) {
    unsafe {
        context_switch(current, next);
    }
}