#![allow(dead_code)]

//! Task management and round-robin scheduler for kernel threads.
//!
//! Preemptive task switching is driven by supervisor timer interrupts.

use crate::arch::riscv64::context::Context;
use crate::arch::riscv64::csr::{SSTATUS_SPIE, SSTATUS_SPP, enable_interrupts};
use crate::arch::riscv64::timer;
use crate::arch::riscv64::trap::{TRAP_FRAME_SIZE, TrapFrame};
use crate::mm::{KMEM, PGSIZE};
use crate::println;

use core::arch::asm;
use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ptr;

/// Maximum number of tasks supported by the kernel at one time.
pub(crate) const MAX_TASKS: usize = 2;

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

/// Global scheduler state and task table.
pub(crate) struct TaskManager {
    /// Fixed-size table of all tasks managed by the kernel.
    tasks: [Task; MAX_TASKS],
    /// Next PID to assign when a new task is created.
    next_pid: TaskId,
    /// PID of the currently executing task, if any.
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

/// Thread-safe wrapper for the global scheduler state.
pub(crate) struct SafeTaskManager(pub(crate) UnsafeCell<TaskManager>);

unsafe impl Sync for SafeTaskManager {}

impl SafeTaskManager {
    /// Returns a mutable reference to the inner `TaskManager`.
    ///
    /// # Safety
    ///
    /// Caller must guarantee exclusive access to the task manager state.
    #[inline]
    pub(crate) fn get_mut(&self) -> &mut TaskManager {
        unsafe { &mut *self.0.get() }
    }

    /// Returns an immutable reference to the inner `TaskManager`.
    ///
    /// # Safety
    ///
    /// Caller must guarantee no concurrent mutable references exist.
    #[inline]
    pub(crate) fn get_ref(&self) -> &TaskManager {
        unsafe { &*self.0.get() }
    }
}

/// Shared singleton instance of the task manager.
pub(crate) static TASK_MANAGER: SafeTaskManager =
    SafeTaskManager(UnsafeCell::new(TaskManager::new()));

/// Returns the PID of the task currently scheduled on the CPU.
pub(crate) fn current_task() -> Option<TaskId> {
    TASK_MANAGER.get_ref().current
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
    let manager = TASK_MANAGER.get_mut();
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

/// Minimal kernel task 1 emitting a heartbeat message.
pub(crate) fn task1() -> ! {
    loop {
        println!("[Task 1] running");
        for _ in 0..4_000_000 {
            spin_loop();
        }
    }
}

/// Minimal kernel task 2 verifying preemptive multitasking.
pub(crate) fn task2() -> ! {
    loop {
        println!("[Task 2] running");
        for _ in 0..4_000_000 {
            spin_loop();
        }
    }
}

/// Initializes the task subsystem and registers initial kernel tasks.
///
/// Allocates private kernel stacks, constructs supervisor contexts, and registers
/// initial tasks into the global task manager. Must be called during system boot.
pub(crate) fn init() {
    let manager = TASK_MANAGER.get_mut();

    // Allocate kernel stacks through updated MM interface.
    let stack1 = KMEM.get_mut().kalloc();
    if stack1.is_null() {
        panic!("scheduler: failed to allocate stack1");
    }

    let stack2 = KMEM.get_mut().kalloc();
    if stack2.is_null() {
        panic!("scheduler: failed to allocate stack2");
    }

    // Set up initial SSTATUS for tasks: SPP=1 (Supervisor mode) and SPIE=1 (Enable interrupts on sret).
    let initial_sstatus = SSTATUS_SPP | SSTATUS_SPIE;

    // Preserve boot-time GP and TP values.
    let boot_gp = read_gp();
    let boot_tp = read_tp();

    let pid1 = manager
        .create_task(
            stack1,
            task1 as *const () as usize,
            boot_gp,
            boot_tp,
            initial_sstatus,
        )
        .expect("task::init: failed to create task1");

    let pid2 = manager
        .create_task(
            stack2,
            task2 as *const () as usize,
            boot_gp,
            boot_tp,
            initial_sstatus,
        )
        .expect("task::init: failed to create task2");

    println!("\nStarting preemptive scheduler...");
    println!("Task 1 PID: {}", pid1);
    println!("Task 2 PID: {}", pid2);
}

/// Starts the preemptive task scheduler and transitions into the first task.
///
/// Enables timer interrupts and performs context bootstrap into Task 1.
/// This function never returns under normal operation.
pub(crate) fn scheduler() -> ! {
    let manager = TASK_MANAGER.get_mut();

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
