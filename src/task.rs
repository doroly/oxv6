//! Task management.
//!
//! This module provides the basic task control structure used
//! by the kernel. It is inspired by xv6's process structure,
//! but currently only implements the basic task infrastructure.
//!
//! Scheduling and context switching will be added in later stages.

use crate::context::Context;
use crate::mm::PGSIZE;

/// Maximum number of tasks supported by the kernel.
pub(crate) const MAX_TASKS: usize = 16;

/// Kernel stack size of each task.
///
/// Each task currently receives one physical 4 KiB page
/// as its kernel stack.
pub(crate) const KERNEL_STACK_SIZE: usize = PGSIZE;

/// Task identifier.
pub(crate) type TaskId = usize;

/// Current state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskState {
    /// Task slot has not been used.
    Unused,

    /// Task has been created and is ready to run.
    Runnable,

    /// Task is currently running.
    Running,

    /// Task has finished execution.
    Zombie,
}

/// A task control block.
///
/// This structure is similar to xv6's `struct proc`.
///
/// It contains the minimum information required to manage
/// a kernel task.
pub(crate) struct Task {
    /// Unique task identifier.
    pub(crate) pid: TaskId,

    /// Current task state.
    pub(crate) state: TaskState,

    /// Base address of the task's kernel stack.
    ///
    /// The stack grows downward on RISC-V.
    pub(crate) kernel_stack: *mut u8,

    /// Saved CPU context used during context switching.
    pub(crate) context: Context,
}

impl Task {
    /// Creates an empty task.
    pub(crate) const fn empty() -> Self {
        Self {
            pid: 0,
            state: TaskState::Unused,
            kernel_stack: core::ptr::null_mut(),
            context: Context::zero(),
        }
    }

    /// Creates a new runnable task.
    ///
    /// `pid` identifies the task.
    ///
    /// `kernel_stack` points to a 4 KiB physical page that will
    /// be used as the task's kernel stack.
    ///
    /// `entry` is the function where the task begins execution.
    ///
    /// # Safety
    ///
    /// - `kernel_stack` must point to a valid writable 4 KiB page.
    /// - The page must remain exclusively owned by this task.
    /// - `entry` must be a valid function address.
    pub(crate) fn new(pid: TaskId, kernel_stack: *mut u8, entry: usize) -> Self {
        let stack_start = kernel_stack as usize;
        let stack_top = stack_start + KERNEL_STACK_SIZE;

        Self {
            pid,
            state: TaskState::Runnable,
            kernel_stack,
            context: Context::new(entry, stack_top),
        }
    }
}

/// Global task manager.
///
/// Currently, this manager only stores task metadata.
/// Scheduling will be implemented in a later stage.
pub(crate) struct TaskManager {
    tasks: [Task; MAX_TASKS],

    next_pid: TaskId,
}

impl TaskManager {
    /// Creates an empty task manager.
    pub(crate) const fn new() -> Self {
        Self {
            tasks: [const { Task::empty() }; MAX_TASKS],

            next_pid: 1,
        }
    }

    /// Creates a new task using a physical memory page
    /// as its kernel stack.
    ///
    /// Returns the task ID on success.
    ///
    /// Returns `None` if all task slots are occupied.
    pub(crate) fn create_task(&mut self, kernel_stack: *mut u8, entry: usize) -> Option<TaskId> {
        for task in self.tasks.iter_mut() {
            if task.state == TaskState::Unused {
                let pid = self.next_pid;

                self.next_pid += 1;

                *task = Task::new(pid, kernel_stack, entry);

                return Some(pid);
            }
        }

        None
    }

    /// Returns a reference to a task by PID.
    pub(crate) fn get(&self, pid: TaskId) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|task| task.state != TaskState::Unused && task.pid == pid)
    }
}

/// Global task manager instance.
///
/// `UnsafeCell` is used because the kernel manages the task
/// table manually without relying on Rust's normal ownership
/// model for global mutable state.
pub(crate) struct SafeTaskManager(pub(crate) core::cell::UnsafeCell<TaskManager>);

unsafe impl Sync for SafeTaskManager {}

/// Global task manager.
pub(crate) static TASK_MANAGER: SafeTaskManager =
    SafeTaskManager(core::cell::UnsafeCell::new(TaskManager::new()));

/// A simple test task.
///
/// This function will later be replaced by a real task entry
/// function when the scheduler is implemented.
pub(crate) fn test_task() -> ! {
    crate::uart::print_str("Task 1 started!\n");

    loop {
        core::hint::spin_loop();
    }
}

/// Test the task infrastructure.
///
/// Creates one task, allocates a kernel stack for it,
/// initializes its CPU context, and prints the task metadata.
pub(crate) fn test_task_manager() {
    use crate::uart::{print_hex, print_str};

    print_str("\nTask Management Test\n");

    // Get a raw pointer to the global task manager.
    let manager = TASK_MANAGER.0.get();

    // Allocate one physical page for the task's kernel stack.
    let kernel_stack = unsafe { (*crate::mm::KMEM.0.get()).kalloc() };

    if kernel_stack.is_null() {
        print_str("Failed to allocate task kernel stack!\n");
        return;
    }

    print_str("Task kernel stack allocated at: ");
    print_hex(kernel_stack as usize);
    print_str("\n");

    // Create the task.
    let pid = unsafe { (*manager).create_task(kernel_stack, test_task as *const () as usize) };

    let pid = match pid {
        Some(pid) => pid,

        None => {
            print_str("Failed to create task!\n");
            return;
        }
    };

    print_str("Task created successfully. PID: ");
    print_hex(pid);
    print_str("\n");

    // Inspect the newly created task.
    unsafe {
        if let Some(task) = (*manager).get(pid) {
            print_str("Task state: Runnable\n");
            print_str("Kernel stack: ");
            print_hex(task.kernel_stack as usize);
            print_str("\nContext SP:");
            print_hex(task.context.sp);
            print_str("\nContext RA: ");
            print_hex(task.context.ra);
            print_str("\n");
        }
    }
}
