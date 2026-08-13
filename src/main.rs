// Disable the Rust standard library (`std`).
#![no_std]
// Disable the default Rust entry point (`main`).
#![no_main]

mod arch;
mod mm;
mod task;
mod uart;

use crate::arch::riscv64::trap;
use crate::uart::{print_hex, print_str};
use core::panic::PanicInfo;

/// Handles unrecoverable kernel errors in a bare-metal environment.
///
/// This panic hook is used by the kernel when a fatal invariant fails or an internal
/// assumption is violated. It prints the panic location and message through the UART
/// console and then halts the CPU in an infinite spin loop.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print_str("\n================ [KERNEL PANIC] ================\n");

    if let Some(location) = info.location() {
        print_str("Location: ");
        print_str(location.file());
        print_str(":");
        print_hex(location.line() as usize);
        print_str(":");
        print_hex(location.column() as usize);
        print_str("\n");
    }

    if let Some(message) = info.message().as_str() {
        print_str("Message:  ");
        print_str(message);
        print_str("\n");
    }

    print_str("================================================\n");

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
    print_str("\n==============================\n");
    print_str("        oxv6 Kernel\n");
    print_str("==============================\n");
    print_str("Privilege Mode: Supervisor\n");

    trap::init();
    mm::kmem_init();
    task::scheduler();
}
