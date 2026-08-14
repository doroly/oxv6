//! Physical memory management for the kernel.
//!
//! Lightweight 4 KiB page allocator using an intrusive singly-linked list
//! stored directly inside free physical memory pages.

use crate::println;
use core::cell::UnsafeCell;
use core::ptr;

/// Size of a physical memory page (4 KiB).
pub(crate) const PGSIZE: usize = 4096;

/// End address of usable physical RAM for the QEMU `virt` platform.
pub(crate) const PHYSTOP: usize = 0x8800_0000;

unsafe extern "C" {
    /// End address of the kernel image, defined in the linker script.
    static ekernel: u8;
}

/// Helper function to retrieve the kernel boundary address.
#[inline]
fn ekernel_addr() -> usize {
    #[allow(unused_unsafe)]
    unsafe { ptr::addr_of!(ekernel) as usize }
}

/// Rounds a physical address up to the next 4 KiB page boundary.
#[inline]
pub fn page_round_up(addr: usize) -> usize {
    (addr + PGSIZE - 1) & !(PGSIZE - 1)
}

/// Intrusive linked-list node stored at the start of each free page.
#[repr(C)]
pub struct Run {
    pub next: *mut Run,
}

/// Physical page allocator based on a singly linked free list.
pub(crate) struct PhysicalMemoryAllocator {
    head: *mut Run,
}

impl PhysicalMemoryAllocator {
    /// Creates an empty allocator instance.
    pub(crate) const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
        }
    }

    /// Populates the free list with every page in range `[start, end)`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory region is valid physical RAM and
    /// does not overlap with the kernel image or MMIO regions.
    pub(crate) fn kinit(&mut self, start: usize, end: usize) {
        let mut page = page_round_up(start);

        while page + PGSIZE <= end {
            self.kfree(page as *mut u8);
            page += PGSIZE;
        }
    }

    /// Returns a physical page to the allocator free list.
    ///
    /// The freed memory is filled with junk (`0x01`) to help detect
    /// use-after-free and dangling pointer bugs during debugging.
    ///
    /// # Panics
    ///
    /// Panics if the address is not page-aligned or lies outside valid physical memory bounds.
    pub(crate) fn kfree(&mut self, pa: *mut u8) {
        let addr = pa as usize;

        if addr % PGSIZE != 0 || addr < ekernel_addr() || addr >= PHYSTOP {
            panic!("kfree: invalid physical address {:#x}", addr);
        }

        unsafe {
            // Fill memory with 0x01 to catch use-after-free bugs.
            ptr::write_bytes(pa, 0x01, PGSIZE);

            // Push the page onto the head of the free list.
            let r = pa as *mut Run;
            (*r).next = self.head;
            self.head = r;
        }
    }

    /// Allocates one 4 KiB physical memory page.
    ///
    /// Returns a pointer to the allocated page filled with `0x05`, or null if out of memory.
    #[inline]
    pub(crate) fn kalloc(&mut self) -> *mut u8 {
        let page = self.head;

        if !page.is_null() {
            unsafe {
                self.head = (*page).next;
                // Fill memory with 0x05 to catch uninitialized reads.
                ptr::write_bytes(page as *mut u8, 0x05, PGSIZE);
            }
        }

        page as *mut u8
    }
}

/// Thread-safe wrapper enabling interior mutability for the global allocator singleton.
pub(crate) struct SafeAllocator(pub(crate) UnsafeCell<PhysicalMemoryAllocator>);

unsafe impl Sync for SafeAllocator {}

/// Global physical memory allocator instance.
pub(crate) static KMEM: SafeAllocator =
    SafeAllocator(UnsafeCell::new(PhysicalMemoryAllocator::new()));

/// Initializes the kernel physical memory allocator.
///
/// Populates the allocator's free list with all physical memory in range `[ekernel, PHYSTOP)`.
pub(crate) fn kmem_init() {
    let start = ekernel_addr();

    unsafe {
        (*KMEM.0.get()).kinit(start, PHYSTOP);
    }

    println!(
        "kmem: physical memory allocator initialized [{:#018x}, {:#018x})",
        start, PHYSTOP
    );
}