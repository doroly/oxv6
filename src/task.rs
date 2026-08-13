#![allow(dead_code)]

//! Task management for a minimal timer-preemptive scheduler.
//!
//! After bootstrapping into the first task, task switching only happens in the timer-interrupt
//! path (`trap::rust_trap_handler -> task::timer_tick`).

use crate::arch::riscv64::context::Context;
use crate::arch::riscv64::csr::{SSTATUS_SPIE, SSTATUS_SPP};
use crate::arch::riscv64::timer;
use crate::arch::riscv64::trap::{TRAP_FRAME_SIZE, TrapFrame};
use crate::mm::{KMEM, PGSIZE};
use crate::uart::{print_hex, print_str};

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

/// A single task-control block.
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

    /// Builds a runnable task with an initial TrapFrame.
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
    fn find_next_runnable_index(&self, current_index: usize) -> Option<usize> {
        for offset in 1..=MAX_TASKS {
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

/// Shared singleton instance of the task manager.
pub(crate) static TASK_MANAGER: SafeTaskManager =
    SafeTaskManager(UnsafeCell::new(TaskManager::new()));

/// Returns the PID of the task currently scheduled on the CPU.
pub(crate) fn current_task() -> Option<TaskId> {
    unsafe { (&*TASK_MANAGER.0.get()).current }
}

/// Called by the trap handler on every supervisor timer interrupt.
///
/// This function implements a simple round-robin scheduler:
///
/// 1. Saves the current task's trap frame and marks it as `Runnable`.
/// 2. Searches for the next runnable task.
/// 3. Switches to that task by returning its trap frame pointer.
///
/// If no other runnable task is found, the current task continues
/// to run.
///
/// # Parameters
///
/// * `frame` - Mutable reference to the current task's trap frame.
///             The trap frame is updated into the task control block
///             before a potential context switch.
///
/// # Returns
///
/// A raw pointer to the trap frame of the task that should continue
/// execution. The caller (assembly trap return path) will restore
/// registers from this frame.
///
/// # Safety
///
/// - The caller must ensure that `frame` points to a valid trap frame.
/// - This function accesses the global task manager through `UnsafeCell`.
/// - The returned pointer must remain valid until the next context switch.
pub(crate) fn timer_tick(frame: &mut TrapFrame) -> *mut TrapFrame {
    let manager = unsafe { &mut *TASK_MANAGER.0.get() };
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
            manager.tasks[current_index].state = TaskState::Running;
            return frame_ptr;
        }
    };

    manager.tasks[next_index].state = TaskState::Running;
    manager.current = Some(manager.tasks[next_index].pid);
    manager.tasks[next_index].trap_frame
}

/// A minimal kernel task that emits a heartbeat message.
pub(crate) fn task1() -> ! {
    loop {
        print_str("[Task 1] running\n");
        for _ in 0..4_000_000 {
            spin_loop();
        }
    }
}

/// A second kernel task used to verify preemptive scheduling.
pub(crate) fn task2() -> ! {
    loop {
        print_str("[Task 2] running\n");
        for _ in 0..4_000_000 {
            spin_loop();
        }
    }
}

/// Starts the kernel’s preemptive scheduler and performs the initial
/// bootstrap into multitasking.
///
/// This function allocates kernel stacks for the first two tasks,
/// constructs their initial execution contexts, registers them in the
/// global task manager, and enables periodic timer interrupts so that
/// preemption can occur. Task 1 is selected as the initial running task,
/// and the function performs a low‑level context switch from the boot
/// environment into that task.
///
/// After the first context switch, all subsequent scheduling is driven
/// by trap handling and timer interrupts. The function never returns; if
/// control ever comes back to it, the scheduler remains in an infinite
/// spin loop.
///
/// # Safety
///
/// This function performs several unsafe operations, including accessing
/// global statics through raw pointers, manually allocating kernel stacks,
/// and executing a low‑level context switch. These operations are required
/// for kernel initialization and must be used with care.
pub(crate) fn scheduler() -> ! {
    // Get a mutable reference to the global task manager.
    let manager = unsafe { &mut *TASK_MANAGER.0.get() };

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

    // Set up the initial SSTATUS for tasks:
    // SPP=1 (Supervisor mode) and SPIE=1 (enable interrupts on return).
    let initial_sstatus = SSTATUS_SPP | SSTATUS_SPIE;

    // Read the current GP and TP registers to pass to tasks.
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
        .expect("scheduler: failed to create task1");
    let pid2 = manager
        .create_task(
            stack2,
            task2 as *const () as usize,
            boot_gp,
            boot_tp,
            initial_sstatus,
        )
        .expect("scheduler: failed to create task2");

    print_str("\nStarting preemptive scheduler...\n");
    print_str("Task 1 PID: ");
    print_hex(pid1);
    print_str("\nTask 2 PID: ");
    print_hex(pid2);
    print_str("\n");

    manager.current = Some(pid1);
    manager
        .tasks
        .iter_mut()
        .find(|task| task.pid == pid1)
        .expect("scheduler: task1 disappeared")
        .state = TaskState::Running;

    // Start periodic timer interrupts before entering the first task.
    timer::set_next_timer();

    // Bootstrap into the first task once; all subsequent switches are trap-frame based.
    let mut boot_context = Context::zero();
    let first_context = Context::new(
        task1 as *const () as usize,
        stack1 as usize + KERNEL_STACK_SIZE,
    );

    // Start to run Task 1 by switching from the boot context to the first task's context.
    crate::arch::riscv64::context::switch(
        &mut boot_context as *mut Context,
        &first_context as *const Context,
    );

    loop {
        spin_loop();
    }
}

#[inline]
fn read_gp() -> usize {
    let value;
    unsafe {
        asm!(
            "mv {}, gp",
            out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

#[inline]
fn read_tp() -> usize {
    let value;
    unsafe {
        asm!(
            "mv {}, tp",
            out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}
