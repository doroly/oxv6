//! Task subsystem module root and task initialization.

pub(crate) mod cpu;
pub(crate) mod proc;
pub(crate) mod scheduler;
pub(crate) mod shell;

use crate::arch::riscv64::csr::{SSTATUS_SPIE, SSTATUS_SPP};
use crate::mm::kstack;
use crate::task::scheduler::TASK_MANAGER;
use core::arch::asm;

/// Initialize the task management subsystem and launch the initial shell task.
///
/// Under the virtual memory model, process kernel stacks are pre-allocated and
/// mapped during page table creation (`kvmmake`). This function sets up task
/// contexts using their designated virtual memory stack addresses rather than
/// allocating dynamic physical frames at runtime.
pub(crate) fn init() {
    TASK_MANAGER
        .lock()
        .create_task(
            kstack(0) as *mut u8,
            shell::shell_task as *const () as usize,
            read_gp(),
            read_tp(),
            SSTATUS_SPP | SSTATUS_SPIE,
        )
        .expect("task::init: failed to create shell task");
}

/// Reads the global pointer (`gp`) register.
#[inline]
fn read_gp() -> usize {
    let value: usize;
    unsafe {
        asm!(
        "mv {}, gp",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
    value
}

/// Reads the thread pointer (`tp`) register.
#[inline]
fn read_tp() -> usize {
    let value: usize;
    unsafe {
        asm!(
        "mv {}, tp",
        out(reg) value,
        options(nomem, nostack, preserves_flags),
        );
    }
    value
}
