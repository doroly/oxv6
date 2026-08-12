//! RISC-V task context.
//!
//! A task context stores the callee-saved registers that must be
//! preserved when switching from one task to another.
//!
//! The actual context switch routine will be implemented in
//! RISC-V assembly in a later stage.

/// Saved CPU context for a RISC-V task.
///
/// According to the RISC-V calling convention, the callee-saved
/// registers are:
///
/// - `ra`
/// - `sp`
/// - `s0` ~ `s11`
///
/// These registers must be preserved across a context switch.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Context {
    /// Return address register.
    pub(crate) ra: usize,

    /// Stack pointer register.
    pub(crate) sp: usize,

    /// Callee-saved register s0 / frame pointer.
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
    /// Creates an empty CPU context.
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

    /// Creates a context for a newly created task.
    ///
    /// The task starts execution at `entry` with `stack_top`
    /// as its initial stack pointer.
    pub(crate) fn new(entry: usize, stack_top: usize) -> Self {
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
