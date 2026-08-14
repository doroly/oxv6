//! Device drivers for the QEMU virt platform.

pub(crate) mod console;
pub(crate) mod irq;

// Re-export
pub(crate) use console::uart;
pub(crate) use irq::plic;
