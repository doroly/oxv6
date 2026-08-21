//! Task subsystem module root containing boot initialization routines and test tasks.

pub(crate) mod cpu;
pub(crate) mod proc;
pub(crate) mod scheduler;

use crate::arch::riscv64::csr::{SSTATUS_SPIE, SSTATUS_SPP};
use crate::mm::KMEM;
use crate::println;
use core::arch::asm;
use core::hint::spin_loop;
use crate::task::scheduler::TASK_MANAGER;

/// Minimal test task 1 demonstrating preemptive kernel thread execution.
pub(crate) fn task1() -> ! {
    loop {
        println!("[Task 1] running");
        for _ in 0..4_000_000 {
            spin_loop();
        }
    }
}

/// Minimal test task 2 verifying context switching routines.
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
    // Allocate kernel stacks through updated MM interface.
    let stack1 = KMEM.lock().kalloc();
    if stack1.is_null() {
        panic!("scheduler: failed to allocate stack1");
    }

    let stack2 = KMEM.lock().kalloc();
    if stack2.is_null() {
        panic!("scheduler: failed to allocate stack2");
    }

    // Set up initial SSTATUS for tasks: SPP=1 (Supervisor mode) and SPIE=1 (Enable interrupts on sret).
    let initial_sstatus = SSTATUS_SPP | SSTATUS_SPIE;

    // Preserve boot-time GP and TP values.
    let boot_gp = read_gp();
    let boot_tp = read_tp();

    let (pid1, pid2) = {
        let mut manager = TASK_MANAGER.lock();
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
        (pid1, pid2)
    };

    println!("\nStarting preemptive scheduler...");
    println!("Task 1 PID: {}", pid1);
    println!("Task 2 PID: {}", pid2);
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
