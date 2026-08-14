// Disable the Rust standard library (`std`).
#![no_std]
// Disable the default Rust entry point (`main`).
#![no_main]

mod arch;
mod drivers;
mod mm;
mod task;
mod sync;

use crate::drivers::{plic, uart};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::arch::riscv64::sbi::sbi_hart_start;
use core::arch::asm;
use arch::riscv64::boot::{_start, MAX_HARTS};

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

/// Flag indicating that primary hart initialization is complete.
static STARTED: AtomicBool = AtomicBool::new(false);
/// Hart elected as primary bootstrap core.
static PRIMARY_HART: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Kernel entry point reached from the assembly startup routine.
///
/// The boot code initializes the stack and branches into this function, which performs
/// early platform setup before handing control to the scheduler. This function never returns.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main(hartid: usize) -> ! {
    let is_primary = PRIMARY_HART
        .compare_exchange(usize::MAX, hartid, Ordering::AcqRel, Ordering::Acquire)
        .is_ok();

    if is_primary {
        // --- Primary Hart (Boot Hart) ---
        uart::init();
        println!("\n==============================");
        println!("        oxv6 Kernel");
        println!("==============================\n");
        println!("Privilege Mode: Supervisor");
        println!("\n[Hart {}] Booting oxv6 multi-core kernel...", hartid);

        plic::init();
        mm::kmem_init();
        task::init();

        println!(
            "[Hart {}] Initialization complete. Waking up secondary harts...",
            hartid
        );

        // Release other Secondary Harts
        STARTED.store(true, Ordering::Release);

        // Send SBI call to wake up secondary harts via OpenSBI HSM extension
        for target_hart in 0..MAX_HARTS {
            if target_hart != hartid {
                sbi_hart_start(target_hart, _start as *const () as usize, 0);
            }
        }
    } else {
        // --- Secondary Harts (Hart 1, 2, 3...) ---
        // Wait for Hart 0 to complete global hardware and memory initialization
        while !STARTED.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }

        println!("[Hart {}] Secondary hart online!", hartid);
    }

    // Per-hart local interrupt routing and trap setup.
    plic::init_hart(hartid);
    arch::riscv64::trap::init();

    if is_primary {
        task::scheduler();
    }

    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
