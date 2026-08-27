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
    set_tp(hartid);

    let is_primary = PRIMARY_HART
        .compare_exchange(usize::MAX, hartid, Ordering::AcqRel, Ordering::Acquire)
        .is_ok();

    if is_primary {
        uart::init();
        println!("xv6 kernel is booting\n");

        plic::init();
        mm::kmem_init();
        timer::init();
        task::init();

        // Announce each secondary in deterministic order before allowing the
        // schedulers to run the shell task.
        for target_hart in 1..MAX_HARTS {
            sbi_hart_start(target_hart, _start as *const () as usize, 0);
            println!("hart {} starting", target_hart);
        }
        STARTED.store(true, Ordering::Release);
    } else {
        while !STARTED.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
    }

    plic::init_hart(hartid);
    trap::init();
    scheduler();
}
