// Disable the Rust standard library (`std`).
#![no_std]
// Disable the default Rust entry point (`main`).
#![no_main]

mod arch;
mod mm;
mod task;
mod uart;

use crate::arch::riscv64::{plic, trap};
use core::panic::PanicInfo;

/// Handles unrecoverable kernel errors in a bare-metal environment.
///
/// This panic hook is used by the kernel when a fatal invariant fails or an internal
/// assumption is violated. It prints the panic location and message through the UART
/// console and then halts the CPU in an infinite spin loop.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n================ [KERNEL PANIC] ================");

    if let Some(location) = info.location() {
        println!(
            "Location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    } else {
        println!("Location: Unknown");
    }

    println!("Message:  {}", info.message());

    println!("================================================\n");

    loop {
        core::hint::spin_loop();
    }
}

/// Kernel entry point reached from the assembly startup routine.
///
/// The boot code initializes the stack and branches into this function, which performs
/// early platform setup before handing control to the scheduler. This function never returns.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    println!("\n==============================");
    println!("        oxv6 Kernel");
    println!("==============================\n");
    println!("Privilege Mode: Supervisor");

    // 1. Initialize physical memory allocator.
    mm::kmem_init();

    // 5. Install the trap handler.
    trap::init();

    // 3. Initialize the Platform-Level Interrupt Controller.
    plic::init();

    // 4. Initialize UART receive interrupts.
    uart::init();

    println!("Hardware & Interrupt subsystem initialized.");

    // 2. Initialize task subsystem.
    task::init();

    // 7. Start task scheduling.
    task::scheduler();
}