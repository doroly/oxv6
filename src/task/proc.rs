#![allow(dead_code)]
//! Task control block and state management.

use crate::arch::riscv64::trap::{TRAP_FRAME_SIZE, TrapFrame};
use crate::mm::PGSIZE;
use core::ptr;

/// Size of each kernel stack, matching one 4 KiB page.
pub(crate) const KERNEL_STACK_SIZE: usize = PGSIZE;

/// Task ID type.
pub(crate) type TaskId = usize;

/// Runtime state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskState {
    /// The slot is currently unused.
    Unused,
    /// The task is ready to run when the scheduler selects it.
    Runnable,
    /// The task is currently executing on the CPU.
    Running,
    /// The task has exited and its slot is retained for bookkeeping.
    Zombie,
}

/// A single task-control block (TCB).
pub(crate) struct Task {
    /// Stable integer identifier assigned at creation time.
    pub(crate) pid: TaskId,
    /// Current execution state for this task.
    pub(crate) state: TaskState,
    /// Base address of the task's private kernel stack.
    pub(crate) kernel_stack: *mut u8,
    /// Saved trap frame pointer used by timer preemption.
    pub(crate) trap_frame: *mut TrapFrame,
}

/// SAFETY: `Task` holds raw pointers (`*mut u8` and `*mut TrapFrame`) which do not
/// implement `Send` by default. Thread safety is guaranteed via `SpinLock` synchronization.
unsafe impl Send for Task {}

impl Task {
    /// Creates a task slot in the unused state.
    pub(crate) const fn empty() -> Self {
        Self {
            pid: 0,
            state: TaskState::Unused,
            kernel_stack: ptr::null_mut(),
            trap_frame: ptr::null_mut(),
        }
    }

    /// Builds a runnable task with an initial `TrapFrame` at the top of its stack.
    pub(crate) fn new(
        pid: TaskId,
        kernel_stack: *mut u8,
        entry: usize,
        gp: usize,
        tp: usize,
        initial_sstatus: usize,
    ) -> Self {
        let stack_start = kernel_stack as usize;
        let stack_top = stack_start + KERNEL_STACK_SIZE;
        let frame_addr = stack_top - TRAP_FRAME_SIZE;
        let frame_ptr = frame_addr as *mut TrapFrame;

        unsafe {
            ptr::write(
                frame_ptr,
                TrapFrame::for_kernel_task(entry, stack_top, gp, tp, initial_sstatus),
            );
        }

        Self {
            pid,
            state: TaskState::Runnable,
            kernel_stack,
            trap_frame: frame_ptr,
        }
    }
}
