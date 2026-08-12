//! Task management.
//!
//! This module provides the basic task control structure used
//! by the kernel. It is inspired by xv6's process structure,
//! but currently only implements the basic task infrastructure.
//!
//! Scheduling and context switching will be added in later stages.

use crate::context::Context;
use crate::mm::{KMEM, PGSIZE};
use crate::uart::{print_hex, print_str};
use core::cell::UnsafeCell;
use core::hint::spin_loop;

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
/// Maintains all task control blocks and records which task
/// is currently running.
pub(crate) struct TaskManager {
    /// Task table.
    tasks: [Task; MAX_TASKS],

    /// PID assigned to the next created task.
    next_pid: TaskId,

    /// Currently running task.
    current: Option<TaskId>,
}

impl TaskManager {
    /// Creates an empty task manager.
    pub(crate) const fn new() -> Self {
        Self {
            tasks: [const { Task::empty() }; MAX_TASKS],
            next_pid: 1,
            current: None,
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

    /// Finds the next runnable task.
    fn find_next_runnable(&self, current: Option<TaskId>) -> Option<TaskId> {
        self.tasks
            .iter()
            .find(|task| task.state == TaskState::Runnable && Some(task.pid) != current)
            .map(|task| task.pid)
    }

    /// Returns a mutable task by PID.
    fn get_mut(&mut self, pid: TaskId) -> Option<&mut Task> {
        self.tasks
            .iter_mut()
            .find(|task| task.state != TaskState::Unused && task.pid == pid)
    }
}

/// Global task manager instance.
///
/// `UnsafeCell` is used because the kernel manages the task
/// table manually without relying on Rust's normal ownership
/// model for global mutable state.
pub(crate) struct SafeTaskManager(pub(crate) UnsafeCell<TaskManager>);

unsafe impl Sync for SafeTaskManager {}

/// Global task manager.
pub(crate) static TASK_MANAGER: SafeTaskManager =
    SafeTaskManager(UnsafeCell::new(TaskManager::new()));

struct BootContext(UnsafeCell<Context>);

unsafe impl Sync for BootContext {}

/// Context used while the kernel is bootstrapping.
///
/// The first context switch leaves `rust_main` and never needs
/// to return to it. The context is nevertheless kept so that
/// the scheduler has a valid location in which to save the
/// boot-time CPU state.
static BOOT_CONTEXT: BootContext = BootContext(UnsafeCell::new(Context::zero()));

/// Voluntarily yield the CPU to another runnable task.
///
/// This is cooperative scheduling: a task must explicitly call
/// `yield_task()` before another task can run.
pub(crate) fn yield_task() {
    // Get a mutable reference to the global task manager.
    let manager = unsafe { &mut *TASK_MANAGER.0.get() };

    // Get the currently running task ID.
    let current_pid = manager.current;

    // Find the next runnable task.
    let next_pid = match (*manager).find_next_runnable(current_pid) {
        Some(pid) => pid,
        None => {
            return;
        }
    };

    // Mark the current task as runnable again.
    if let Some(pid) = current_pid {
        if let Some(task) = (*manager).get_mut(pid) {
            task.state = TaskState::Runnable;
        }
    }

    // Mark the next task as running.
    if let Some(task) = (*manager).get_mut(next_pid) {
        task.state = TaskState::Running;
    }

    // Save current task context and restore next task.
    let current_context = match current_pid {
        Some(pid) => (*manager)
            .get_mut(pid)
            .map(|task| &mut task.context as *mut Context)
            .unwrap(),
        None => BOOT_CONTEXT.0.get(),
    };

    // Get the next task's context to restore.
    let next_context = (*manager)
        .get(next_pid)
        .map(|task| &task.context as *const Context)
        .unwrap();

    // Update the current task ID to the next task.
    (*manager).current = Some(next_pid);

    unsafe {
        // Perform the context switch.
        context_switch(current_context, next_context);
    }
}

/// First kernel task.
pub(crate) fn task1() -> ! {
    loop {
        print_str("[Task 1] running\n");

        for _ in 0..1_000_000 {
            spin_loop();
        }

        yield_task();
    }
}

/// Second kernel task.
pub(crate) fn task2() -> ! {
    loop {
        print_str("[Task 2] running\n");

        for _ in 0..1_000_000 {
            spin_loop();
        }

        yield_task();
    }
}

unsafe extern "C" {
    pub(crate) fn context_switch(current: *mut Context, next: *const Context);
}

/// Start the cooperative scheduler.
///
/// Two kernel tasks are created and the first task is started
/// through the RISC-V context switch mechanism.
pub(crate) fn scheduler() -> ! {
    // Get a mutable reference to the global task manager.
    let manager = TASK_MANAGER.0.get();

    // Allocate a kernel stack for Task 1.
    let stack1 = unsafe { (*KMEM.0.get()).kalloc() };

    if stack1.is_null() {
        panic!("scheduler: failed to allocate stack1");
    }

    // Allocate a kernel stack for Task 2.
    let stack2 = unsafe { (*KMEM.0.get()).kalloc() };

    if stack2.is_null() {
        panic!("scheduler: failed to allocate stack2");
    }

    // Create Task 1.
    let pid1 = unsafe {
        (*manager)
            .create_task(stack1, task1 as *const () as usize)
            .expect("scheduler: failed to create task1")
    };

    // Create Task 2.
    let pid2 = unsafe {
        (*manager)
            .create_task(stack2, task2 as *const () as usize)
            .expect("scheduler: failed to create task2")
    };

    print_str("\nStarting cooperative scheduler...\n");
    print_str("Task 1 PID: ");
    print_hex(pid1);
    print_str("\nTask 2 PID: ");
    print_hex(pid2);
    print_str("\n");

    unsafe {
        // Start Task 1.
        (*manager).current = Some(pid1);

        (*manager).get_mut(pid1).unwrap().state = TaskState::Running;

        let next_context = &(*manager).get(pid1).unwrap().context as *const Context;

        // Save the boot context and restore Task 1.
        context_switch(BOOT_CONTEXT.0.get(), next_context);
    }

    // Execution should never normally return here.
    loop {
        spin_loop();
    }
}
