//! Physical memory management module.
//!
//! Implements a simple page allocator similar to xv6.
//! Each free physical page stores a linked-list node inside
//! its own memory area, avoiding additional allocation.

use core::cell::UnsafeCell;
use core::ptr;

/// Page size in bytes (4 KiB).
pub(crate) const PGSIZE: usize = 4096;

/// Top of physical memory (end address of usable RAM).
///
/// QEMU `virt` machine provides 128 MiB physical memory in range `[0x8000_0000, 0x8800_0000)`.
pub(crate) const PHYSTOP: usize = 0x8800_0000;

unsafe extern "C" {
    /// End address of the kernel image, defined in `kernel.ld`.
    static ekernel: u8;
}

/// Rounds up the given memory address to the nearest 4KB (PGSIZE) page boundary.
///
/// # Logic
/// 1. Adds `(PGSIZE - 1)` to trigger a carry into the page frame number bits
///    if `addr` is not already page-aligned.
/// 2. Clears the lower 12 bits (page offset) using bitwise AND with the bitmask `!(PGSIZE - 1)`.
pub fn page_round_up(addr: usize) -> usize {
    (addr + PGSIZE - 1) & !(PGSIZE - 1)
}

/// A node in the intrusive free memory page linked list.
///
/// This structure is placed directly at the beginning of an unallocated
/// 4KB physical memory page, allowing the allocator to maintain a free list
/// with zero extra memory overhead.
#[repr(C)]
pub struct Run {
    /// Raw pointer pointing to the next available physical page (`Run` node).
    /// Holds a null pointer if this is the end of the free list.
    pub next: *mut Run,
}

/// A physical memory page allocator.
///
/// Manages free 4 KiB memory pages using an intrusive singly-linked list.
pub(crate) struct PhysicalMemoryAllocator {
    /// Pointer to the head of the free page list.
    head: *mut Run,
}

impl PhysicalMemoryAllocator {
    /// Creates a new, uninitialized physical memory allocator.
    pub(crate) const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
        }
    }

    /// Initializes the physical memory allocator with a range of memory addresses.
    ///
    /// Divides the memory range `[start, end)` into 4 KiB pages and adds them to the free list.
    ///
    /// # Safety
    ///
    /// - `start` and `end` must represent a valid range of physical memory.
    /// - Memory in `[start, end)` must be writable and not used by the kernel image or hardware devices.
    pub(crate) fn kinit(&mut self, start: usize, end: usize) {
        // Align the starting address up to the nearest 4 KiB page boundary.
        let mut p = page_round_up(start);

        // Iterate over every page-sized block in the range and free it.
        while p + PGSIZE <= end {
            self.kfree(p as *mut u8);

            p += PGSIZE;
        }
    }

    /// Frees a physical memory page, returning it to the allocator.
    ///
    /// The freed page is prepended to the head of the free list.
    ///
    /// # Panics
    ///
    /// Panics if `pa` is not aligned to `PGSIZE` (4 KiB) or falls outside the valid range `[ekernel, PHYSTOP)`.
    ///
    /// # Safety
    ///
    /// - `pa` must point to a valid, page-aligned physical address.
    /// - The memory block at `pa` must no longer be referenced or used anywhere else in the system.
    pub(crate) fn kfree(&mut self, pa: *mut u8) {
        let addr = pa as usize;
        #[allow(unused_unsafe)]
        let ekernel_addr = unsafe { ptr::addr_of!(ekernel) as usize };

        // Validate that the physical address is page-aligned and within allowable memory bounds.
        if addr % PGSIZE != 0 || addr < ekernel_addr || addr >= PHYSTOP {
            panic!("kfree: invalid physical address");
        }

        // Fill freed memory with junk (0x01) to catch use-after-free bugs.
        unsafe {
            ptr::write_bytes(pa, 1, PGSIZE);
        }

        let r = pa as *mut Run;

        // Push the freed page onto the head of the singly-linked list.
        unsafe {
            (*r).next = self.head;
            self.head = r;
        }
    }

    /// Allocates a single 4 KiB physical memory page.
    ///
    /// Returns a raw pointer to the allocated page, or null if no pages are available.
    ///
    /// # Safety
    ///
    /// The caller assumes ownership of the allocated page and must ensure accesses stay within 4 KiB bounds.
    pub(crate) fn kalloc(&mut self) -> *mut u8 {
        let r = self.head;

        if !r.is_null() {
            // Pop the top page off the free list.
            unsafe {
                self.head = (*r).next;
                // Fill allocated memory with junk (0x05) to catch uninitialized memory reads.
                ptr::write_bytes(r as *mut u8, 5, PGSIZE);
            }
        }

        r as *mut u8
    }
}
/// A wrapper around [`UnsafeCell`] that enables global interior mutability for the physical memory allocator.
///
/// In Rust, `static` variables are immutable when accessed across threads/cores.
/// Wrapping [`PhysicalMemoryAllocator`] in [`UnsafeCell`] allows mutating the allocator
/// state through a shared reference.
pub(crate) struct SafeAllocator(pub(crate) UnsafeCell<PhysicalMemoryAllocator>);

/// # Safety
///
/// Implementing [`Sync`] asserts to the compiler that `SafeAllocator` can be safely shared
/// across CPU cores (harts).
///
/// The caller/kernel must ensure that accesses to the underlying [`PhysicalMemoryAllocator`]
/// are synchronized (e.g., via spinlocks, disabling interrupts, or single-core initialization)
/// to prevent data races and undefined behavior.
unsafe impl Sync for SafeAllocator {}

/// The global singleton instance of the physical memory allocator.
pub(crate) static KMEM: SafeAllocator =
    SafeAllocator(UnsafeCell::new(PhysicalMemoryAllocator::new()));

/// Initializes the global physical memory allocator.
///
/// Populates the free list with all available 4 KiB physical pages in the range
/// `[ekernel, PHYSTOP)`.
///
/// # Safety
///
/// - Must be called exactly once during early single-hart kernel boot sequence.
/// - Must be executed after memory layout (e.g., BSS section) is set up and before
///   any physical page allocations take place.
pub(crate) fn kmem_init() {
    use crate::uart::{print_hex, print_str};

    // Obtain the boundary address marking the end of the kernel image.
    #[allow(unused_unsafe)]
    let ekernel_addr = unsafe { ptr::addr_of!(ekernel) as usize };

    // Retrieve raw pointer to PhysicalMemoryAllocator from the UnsafeCell wrapper.
    let kmem_ptr = KMEM.0.get();

    // Populate the free list with memory in range [ekernel_addr, PHYSTOP).
    unsafe {
        (*kmem_ptr).kinit(ekernel_addr, PHYSTOP);
    }

    // Output initialization log to UART console
    print_str("kmem: physical memory allocator initialized [");
    print_hex(ekernel_addr);
    print_str(", ");
    print_hex(PHYSTOP);
    print_str(")\n");
}
