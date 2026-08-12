// Disable the Rust standard library (`std`).
#![no_std]
// Disable the default Rust entry point (`main`).
#![no_main]

mod context;
mod mm;
mod task;
mod uart;

use core::arch::global_asm;
use core::panic::PanicInfo;

// Embed the boot assembly code.
//
// This code initializes the boot stack and transfers execution
// to the Rust entry point (`rust_main`).
global_asm!(
    r#"
.section .text.entry          # Place startup code in the .text.entry section
.globl _start                 # Export the _start symbol

_start:
    la sp, boot_stack_top     # Load the boot stack top address into sp
    call rust_main            # Jump to the Rust entry point

loop:
    j loop                    # Halt here if rust_main unexpectedly returns


.section .bss.stack           # Reserve stack space in the .bss section
.globl boot_stack_lower_bound # Export the stack lower bound symbol

boot_stack_lower_bound:
    .space 4096 * 4            # Reserve 16 KiB for the boot stack

.globl boot_stack_top          # Export the stack top symbol
boot_stack_top:
"#
);

/// Handles panic events in a bare-metal environment.
///
/// In a `no_std` environment, there is no operating system to handle
/// panic messages. This handler stops program execution by entering
/// an infinite loop.
///
/// # Arguments
///
/// * `_info` - Contains panic information, such as the source location
///   and panic message.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// Rust application entry point.
///
/// This function is called by the assembly startup code after the
/// stack pointer has been initialized.
///
/// The function uses the C calling convention and never returns.
///
/// # Safety
///
/// This function is invoked directly by the hardware startup code.
/// The caller must ensure that the runtime environment has been
/// properly initialized before calling this function.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    // Initialize the physical memory allocator
    mm::kmem_init();

    // Initialize and test the task infrastructure.
    task::scheduler();

}
