#![allow(dead_code)]

//! Task management and round-robin scheduler for kernel threads.
//!
//! Preemptive task switching is driven by supervisor timer interrupts.

use crate::arch::riscv64::context::Context;
use crate::arch::riscv64::csr::enable_interrupts;
use crate::arch::riscv64::timer;
use crate::arch::riscv64::trap::TrapFrame;
use crate::sync::SpinLock;
use crate::task::proc::{KERNEL_STACK_SIZE, Task, TaskId, TaskState};
use crate::task::task1;
use core::hint::spin_loop;

/// Maximum number of tasks supported by the kernel at one time.
pub(crate) const MAX_TASKS: usize = 2;

/// Global scheduler state and task table.
pub(crate) struct TaskManager {
    /// Fixed-size table of all tasks managed by the kernel.
    tasks: [Task; MAX_TASKS],
    /// Next PID to assign when a new task is created.
    next_pid: TaskId,
    /// PID of the currently executing task, if any.
    current: Option<TaskId>,
}

/// SAFETY: Synchronized access to `TaskManager` via `SpinLock` prevents
/// data races across multiple harts.
unsafe impl Send for TaskManager {}

impl TaskManager {
    /// Creates an empty task manager.
    pub(crate) const fn new() -> Self {
        Self {
            tasks: [const { Task::empty() }; MAX_TASKS],
            next_pid: 1,
            current: None,
        }
    }

    /// Allocates a task slot and initializes a runnable task.
    pub(crate) fn create_task(
        &mut self,
        kernel_stack: *mut u8,
        entry: usize,
        gp: usize,
        tp: usize,
        initial_sstatus: usize,
    ) -> Option<TaskId> {
        for task in self.tasks.iter_mut() {
            if task.state == TaskState::Unused {
                let pid = self.next_pid;
                self.next_pid += 1;
                *task = Task::new(pid, kernel_stack, entry, gp, tp, initial_sstatus);
                return Some(pid);
            }
        }
        None
    }

    /// Returns a task reference by PID.
    pub(crate) fn get(&self, pid: TaskId) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|task| task.state != TaskState::Unused && task.pid == pid)
    }

    /// Returns the table index for the given PID.
    fn index_of_pid(&self, pid: TaskId) -> Option<usize> {
        self.tasks
            .iter()
            .position(|task| task.state != TaskState::Unused && task.pid == pid)
    }

    /// Selects the next runnable task index in round-robin order.
    /// Searches strictly among other tasks (excluding the current index).
    fn find_next_runnable_index(&self, current_index: usize) -> Option<usize> {
        for offset in 1..MAX_TASKS {
            let index = (current_index + offset) % MAX_TASKS;
            if self.tasks[index].state == TaskState::Runnable {
                return Some(index);
            }
        }
        None
    }
}

/// Global task manager guarded by a spinlock.
pub(crate) static TASK_MANAGER: SpinLock<TaskManager> =
    SpinLock::new("task_manager", TaskManager::new());

/// Returns the PID of the task currently scheduled on the CPU.
pub(crate) fn current_task() -> Option<TaskId> {
    TASK_MANAGER.lock().current
}

/// Called by the trap handler on every supervisor timer interrupt.
///
/// Implements a round-robin scheduler.
///
/// # Safety
///
/// - `frame` must point to a valid `TrapFrame` allocated on the current task's kernel stack.
/// - Returns a raw pointer to a valid `TrapFrame` that will be restored by `trap_return`.
pub(crate) fn timer_tick(frame: &mut TrapFrame) -> *mut TrapFrame {
    let mut manager = TASK_MANAGER.lock();
    let frame_ptr = frame as *mut TrapFrame;

    let current_pid = match manager.current {
        Some(pid) => pid,
        None => return frame_ptr,
    };

    let current_index = match manager.index_of_pid(current_pid) {
        Some(index) => index,
        None => return frame_ptr,
    };

    manager.tasks[current_index].trap_frame = frame_ptr;
    manager.tasks[current_index].state = TaskState::Runnable;

    let next_index = match manager.find_next_runnable_index(current_index) {
        Some(index) => index,
        None => {
            // No other runnable tasks found; resume execution of current task.
            manager.tasks[current_index].state = TaskState::Running;
            return frame_ptr;
        }
    };

    manager.tasks[next_index].state = TaskState::Running;
    manager.current = Some(manager.tasks[next_index].pid);
    manager.tasks[next_index].trap_frame
}

/// Starts the preemptive task scheduler and transitions into the first task.
///
/// Enables timer interrupts and performs context bootstrap into Task 1.
/// This function never returns under normal operation.
pub(crate) fn scheduler() -> ! {
    let stack1 = {
        let mut manager = TASK_MANAGER.lock();
        let first_task = manager
            .tasks
            .first_mut()
            .expect("scheduler: no tasks registered");

        let pid1 = first_task.pid;
        let stack1 = first_task.kernel_stack;

        manager.current = Some(pid1);
        manager
            .tasks
            .iter_mut()
            .find(|task| task.pid == pid1)
            .expect("scheduler: task1 disappeared")
            .state = TaskState::Running;

        stack1
    };

    // Start periodic timer interrupts before entering the first task.
    timer::set_next_timer();
    enable_interrupts();

    // Bootstrap context switch into Task 1.
    let mut boot_context = Context::zero();
    let first_context = Context::new(
        task1 as *const () as usize,
        stack1 as usize + KERNEL_STACK_SIZE,
    );

    crate::arch::riscv64::context::switch(
        &mut boot_context as *mut Context,
        &first_context as *const Context,
    );

    loop {
        spin_loop();
    }
}
