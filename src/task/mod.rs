//! Task management and scheduling subsystem.

pub(crate) mod manager;

// Re-export task management interface for seamless external usage
#[allow(unused)]
pub(crate) use manager::{
    KERNEL_STACK_SIZE, MAX_TASKS, TASK_MANAGER, Task, TaskId, TaskManager, TaskState, current_task,
    init, scheduler, timer_tick,
};
