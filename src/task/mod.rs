//! Task subsystem module root and task initialization.

pub(crate) mod cpu;
pub(crate) mod proc;
pub(crate) mod scheduler;
pub(crate) mod shell;

use crate::arch::riscv64::csr::{SSTATUS_SPIE, SSTATUS_SPP};
use crate::mm::KMEM;
use crate::task::scheduler::TASK_MANAGER;
use core::arch::asm;

/// Initializes the task subsystem and registers the interactive shell task.
///
/// Allocates a private kernel stack, constructs a supervisor context, and
/// registers the shell as the only initial user-space task.
pub(crate) fn init() {
    let shell_stack = KMEM.lock().kalloc();
    if shell_stack.is_null() {
        panic!("task::init: failed to allocate shell stack");
    }

    let initial_sstatus = SSTATUS_SPP | SSTATUS_SPIE;
    let boot_gp = read_gp();
    let boot_tp = read_tp();

    let mut manager = TASK_MANAGER.lock();
    manager
        .create_task(
            shell_stack,
            shell::shell_task as *const () as usize,
            boot_gp,
            boot_tp,
            initial_sstatus,
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
