//! Memory management subsystem for the kernel.
//!
//! Handles physical frame allocation, virtual page table management (SV39),
//! and kernel dynamic heap allocation.

pub(crate) mod frame_allocator;

// Re-export physical memory allocator interface for seamless external usage
#[allow(unused)]
pub(crate) use frame_allocator::{
    KMEM, PGSIZE, PHYSTOP, PhysicalMemoryAllocator, ekernel_addr, kmem_init, page_round_down,
    page_round_up,
};
