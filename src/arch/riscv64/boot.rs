#![allow(dead_code)]

//! RISC-V startup assembly used to enter the kernel.
//!
//! This module establishes the initial stack pointer and transfers control to the Rust entry
//! routine. The code runs before the kernel has initialized its memory allocator or scheduler.

pub const MAX_HARTS: usize = 4;
pub const BOOT_STACK_SIZE: usize = 16384; // 16 KiB per hart

core::arch::global_asm!(
    r#"
    .section .text.entry          # Place startup code in the .text.entry section
    .globl _start                 # Export the _start symbol
    .type _start, @function       # Declare _start as a function

_start:
    mv tp, a0                     # Preserve hartid in tp for per-hart helpers
    la sp, boot_stack_lower_bound # Base of boot stack array
    slli t0, a0, 14               # hartid * 16 KiB
    li t1, 16384
    add sp, sp, t0
    add sp, sp, t1                # Move to this hart's stack top
    call rust_main                # Jump to the Rust entry point

1:
    j 1b                          # rust_main must never return.

    .section .bss.stack           # Reserve stack space in the .bss section
    .align 12

    .globl boot_stack_lower_bound # Export the stack lower bound symbol

boot_stack_lower_bound:
    .space 4096 * 4 * 4           # Reserve 16 KiB per hart (MAX_HARTS=4)

    .globl boot_stack_top         # Export the stack top symbol

boot_stack_top:
"#
);

unsafe extern "C" {
    /// Assembly entry point symbol defined in `global_asm!`.
    pub fn _start();
}
