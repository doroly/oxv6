//! Physical memory management for the kernel.
//!
//! The allocator is intentionally lightweight: each free 4 KiB page stores a linked-list node
//! in its own memory region, so no extra metadata blocks are required.

use core::cell::UnsafeCell;
use core::ptr;

/// The size of a physical page in bytes. The kernel uses 4 KiB pages.
pub(crate) const PGSIZE: usize = 4096;

/// End address of usable physical RAM for the QEMU `virt` machine.
pub(crate) const PHYSTOP: usize = 0x8800_0000;

unsafe extern "C" {
    /// Ending address of the kernel image, defined in the linker script.
    static ekernel: u8;
}

/// Rounds an address up to the next page boundary.
#[inline]
pub fn page_round_up(addr: usize) -> usize {
    (addr + PGSIZE - 1) & !(PGSIZE - 1)
}

/// A free-page node stored at the beginning of each free physical page.
#[repr(C)]
pub struct Run {
    /// Pointer to the next free page in the intrusive list.
    pub next: *mut Run,
}

/// Physical page allocator built on a singly linked free list.
pub(crate) struct PhysicalMemoryAllocator {
    /// Head of the free-page list.
    head: *mut Run,
}

impl PhysicalMemoryAllocator {
    /// Creates an allocator before any pages are registered.
    pub(crate) const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
        }
    }

    /// Populates the free list with every page in `[start, end)`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the region is valid physical RAM and does not overlap the kernel
    /// image or device memory required by the platform.
    pub(crate) fn kinit(&mut self, start: usize, end: usize) {
        // Align the starting address up to the nearest 4 KiB page boundary.
        let mut page = page_round_up(start);

        // Iterate over every page-sized block in the range and free it.
        while page + PGSIZE <= end {
            self.kfree(page as *mut u8);
            page += PGSIZE;
        }
    }

    /// Returns a page to the allocator.
    ///
    /// The page is inserted at the head of the free list and is filled with a non-zero pattern to
    /// help reveal dangling-pointer and uninitialized-memory bugs during debugging.
    ///
    /// # Panics
    ///
    /// Panics when the address is not page-aligned or is outside the valid heap range.
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
            ptr::write_bytes(pa, 0x01, PGSIZE);
            let r = pa as *mut Run;
            (*r).next = self.head;
            self.head = r;
        }
    }

    /// Allocates one free 4 KiB page.
    ///
    /// On success, the returned pointer owns a full page. The caller is responsible for using it
    /// with the correct alignment and lifetime semantics.
    #[inline]
    pub(crate) fn kalloc(&mut self) -> *mut u8 {
        let page = self.head;

        if !page.is_null() {
            unsafe {
                self.head = (*page).next;
                ptr::write_bytes(page as *mut u8, 0x05, PGSIZE);
            }
        }

        page as *mut u8
    }
}

/// Wrapper that enables interior mutability for the global allocator singleton.
pub(crate) struct SafeAllocator(pub(crate) UnsafeCell<PhysicalMemoryAllocator>);

/// # Safety
///
/// `SafeAllocator` may be shared across CPU harts only if the surrounding code serializes access
/// to the underlying allocator. This project uses a single-hart boot path during initialization.
unsafe impl Sync for SafeAllocator {}

/// Global physical memory allocator for the kernel.
pub(crate) static KMEM: SafeAllocator =
    SafeAllocator(UnsafeCell::new(PhysicalMemoryAllocator::new()));

/// Initializes the kernel’s physical memory allocator.
///
/// This function determines the end address of the kernel image and uses it
/// as the starting point for the free physical memory region. It then invokes
/// the allocator’s `kinit` routine to populate the free list with all memory
/// in the range `[ekernel, PHYSTOP)`. After initialization, it prints the
/// memory range used to set up the allocator for debugging purposes.
///
/// # Safety
///
/// This function performs unsafe operations when accessing the global memory
/// allocator and invoking `kinit`, which manipulates raw physical memory.
/// These operations must be used carefully to maintain memory correctness.
///
/// # Effects
///
/// After completion, the global physical memory allocator is ready to serve
/// kernel memory allocation requests.
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

    print_str("kmem: physical memory allocator initialized [");
    print_hex(ekernel_addr);
    print_str(", ");
    print_hex(PHYSTOP);
    print_str(")\n");
}
