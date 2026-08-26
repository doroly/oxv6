// Disable the Rust standard library (`std`).
#![no_std]
// Disable the default Rust entry point (`main`).
#![no_main]

mod arch;
mod drivers;
mod mm;
mod sync;
mod task;

use crate::arch::riscv64::boot::{_start, MAX_HARTS};
use crate::arch::riscv64::sbi::sbi_hart_start;
use crate::arch::riscv64::{timer, trap};
use crate::drivers::{plic, uart};
use crate::task::scheduler::scheduler;
use core::arch::asm;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Handles unrecoverable kernel errors in a bare-metal environment.
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

/// Writes current Hart ID into thread pointer (`tp`) register.
#[inline]
fn set_tp(hartid: usize) {
    unsafe {
        asm!("mv tp, {}", in(reg) hartid, options(nomem, nostack, preserves_flags));
    }
}

/// Kernel entry point reached from assembly startup routine.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main(hartid: usize) -> ! {
    // Bind Hart ID to thread pointer register for Per-CPU context tracking
    set_tp(hartid);

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
        timer::init();
        task::init();

        println!(
            "[Hart {}] Initialization complete. Waking up secondary harts...",
            hartid
        );

        STARTED.store(true, Ordering::Release);

        // Send SBI call to wake up secondary harts via OpenSBI HSM extension
        for target_hart in 0..MAX_HARTS {
            if target_hart != hartid {
                sbi_hart_start(target_hart, _start as *const () as usize, 0);
            }
        }
    } else {
        // --- Secondary Harts ---
        while !STARTED.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }

        println!("[Hart {}] Secondary hart online!", hartid);
    }

    plic::init_hart(hartid);
    trap::init();

    println!("[Hart {}] Entering scheduler loop...", hartid);

    // All primary and secondary harts enter scheduler loop concurrently
    scheduler();
}
