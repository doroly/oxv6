#![allow(dead_code)]
//! Per-core CPU management and context tracking for RISC-V multiprogramming.

use crate::arch::riscv64::context::Context;
use crate::sync::SpinLock;
use crate::task::proc::TaskId;
use core::arch::asm;

/// Maximum supported CPU cores in system.
pub(crate) const MAX_CPUS: usize = 8;

/// Per-CPU hardware state and scheduler context matching xv6 `struct cpu`.
pub(crate) struct Cpu {
    /// PID of the task currently executing on this CPU core.
    pub(crate) current_task: Option<TaskId>,
    /// CPU core context for switching back to main scheduler loop.
    pub(crate) scheduler_context: Context,
}

impl Cpu {
    /// Creates an initialized CPU core state.
    pub(crate) const fn new() -> Self {
        Self {
            current_task: None,
            scheduler_context: Context::zero(),
        }
    }
}

/// Array of all per-CPU structures guarded by SpinLock for safe concurrent access.
pub(crate) static CPUS: SpinLock<[Cpu; MAX_CPUS]> =
    SpinLock::new("cpus", [const { Cpu::new() }; MAX_CPUS]);

/// Reads current Hart ID from the thread pointer (`tp`) register.
#[inline]
pub(crate) fn mycpu_id() -> usize {
    let id: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) id, options(nomem, nostack, preserves_flags));
    }
    id
}
