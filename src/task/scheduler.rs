//! Task management and round-robin scheduler for kernel threads.

use crate::arch::riscv64::context::{Context, switch};
use crate::arch::riscv64::csr::enable_interrupts;
use crate::arch::riscv64::timer;
use crate::arch::riscv64::trap::TrapFrame;
use crate::sync::SpinLock;
use crate::task::cpu::{CPUS, mycpu_id};
use crate::task::proc::{Task, TaskId, TaskState};
use core::hint::spin_loop;

/// Maximum number of tasks supported by the kernel at one time.
pub(crate) const MAX_TASKS: usize = 16;

/// Global scheduler state and task table.
pub(crate) struct TaskManager {
    /// Fixed-size table of all tasks managed by the kernel.
    tasks: [Task; MAX_TASKS],
    /// Next PID to assign when a new task is created.
    next_pid: TaskId,
}

unsafe impl Send for TaskManager {}

impl TaskManager {
    /// Creates an empty task manager.
    pub(crate) const fn new() -> Self {
        Self {
            tasks: [const { Task::empty() }; MAX_TASKS],
            next_pid: 1,
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
}

/// Global task manager guarded by a spinlock.
pub(crate) static TASK_MANAGER: SpinLock<TaskManager> =
    SpinLock::new("task_manager", TaskManager::new());

/// Relinquishes current CPU core execution and returns to the scheduler.
/// Called upon timer interrupt preemption (xv6 `yield`).
pub(crate) fn yield_current_task() {
    let hartid = mycpu_id();

    // Get currently running task PID on this CPU
    let current_pid = {
        let mut cpus = CPUS.lock();
        let pid = cpus[hartid].current_task;
        cpus[hartid].current_task = None;
        pid
    };

    if let Some(pid) = current_pid {
        let mut manager = TASK_MANAGER.lock();

        // Find task and update its context pointer and state
        if let Some(task) = manager.tasks.iter_mut().find(|t| t.pid == pid) {
            let task_ctx_ptr = &mut task.context as *mut Context;
            drop(manager);

            let cpus = CPUS.lock();
            let cpu_ctx_ptr = &cpus[hartid].scheduler_context as *const Context;
            drop(cpus);

            // Context switch from Task back to CPU Scheduler loop
            switch(task_ctx_ptr, cpu_ctx_ptr);

            return;
        }
        drop(manager);
    }
}

/// Timer interrupt handler for task preemption.
pub(crate) fn timer_tick(frame: &mut TrapFrame) -> *mut TrapFrame {
    // Trigger preemption yield on timer tick
    yield_current_task();
    frame.tp = mycpu_id();
    frame as *mut TrapFrame
}

/// Preemptive multi-core scheduler loop following xv6 design principles.
/// Executed by every CPU Hart concurrently.
pub(crate) fn scheduler() -> ! {
    let hartid = mycpu_id();
    timer::set_next_timer();
    enable_interrupts();

    loop {
        let mut manager = TASK_MANAGER.lock();
        let mut runnable_task: Option<(*const Context, usize)> = None;
        let mut task_idx = 0;

        // Iterate over global process table to find a runnable task
        for (i, task) in manager.tasks.iter_mut().enumerate() {
            if task.state == TaskState::Runnable {
                task.state = TaskState::Running;
                runnable_task = Some((&task.context as *const Context, task.pid));
                task_idx = i;

                // Record the currently running PID on this CPU core
                let mut cpus = CPUS.lock();
                cpus[hartid].current_task = Some(task.pid);
                drop(cpus);
                break;
            }
        }

        if let Some((task_ctx_ptr, _pid)) = runnable_task {
            // Drop TASK_MANAGER lock before switching context to prevent deadlock
            drop(manager);

            let mut cpus = CPUS.lock();
            let cpu_ctx_ptr = &mut cpus[hartid].scheduler_context as *mut Context;
            drop(cpus);

            // Perform context switch from scheduler loop into kernel task
            switch(cpu_ctx_ptr, task_ctx_ptr);

            // Clear current_task on this CPU core after task switches back or completes
            let mut cpus = CPUS.lock();
            cpus[hartid].current_task = None;
            drop(cpus);

            // Re-mark task as Runnable for subsequent scheduling rounds
            let mut manager = TASK_MANAGER.lock();
            if manager.tasks[task_idx].state == TaskState::Running {
                manager.tasks[task_idx].state = TaskState::Runnable;
            }
            drop(manager);
        } else {
            // Release lock and spin-loop when no runnable task is found
            drop(manager);
            spin_loop();
        }
    }
}
