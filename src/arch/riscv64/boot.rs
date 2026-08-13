//! RISC-V startup assembly used to enter the kernel.
//!
//! This module establishes the initial stack pointer and transfers control to the Rust entry
//! routine. The code runs before the kernel has initialized its memory allocator or scheduler.

core::arch::global_asm!(
    r#"
    .section .text.entry          # Place startup code in the .text.entry section
    .globl _start                 # Export the _start symbol
    .type _start, @function       # Declare _start as a function

_start:
    la sp, boot_stack_top         # Load the boot stack top address into sp
    call rust_main                # Jump to the Rust entry point

1:
    j 1b                          # rust_main must never return.

    .section .bss.stack           # Reserve stack space in the .bss section
    .align 12

    .globl boot_stack_lower_bound # Export the stack lower bound symbol

boot_stack_lower_bound:
    .space 4096 * 4               # Reserve 16 KiB for the boot stack

    .globl boot_stack_top         # Export the stack top symbol

boot_stack_top:
"#
);
