//! Memory management subsystem for the kernel.
//!
//! Handles physical frame allocation, virtual page table management (SV39),
//! and kernel dynamic heap allocation.

pub(crate) mod frame_allocator;

// Re-export physical memory allocator interface for seamless external usage
pub(crate) use frame_allocator::{KMEM, PGSIZE, PHYSTOP, kmem_init, page_round_down};

use crate::arch::riscv64::csr;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Page Table Entry format: raw 64-bit integer encoding Physical Page Number (PPN) and flags.
pub(crate) type Pte = usize;

/// A Sv39 page table consists of 512 entries (each 8 bytes, totaling 4096 bytes per page).
pub(crate) type PageTable = [Pte; 512];

/// Maximum number of processes/tasks supported by the system (xv6 default).
pub(crate) const NPROC: usize = 64;

/// Kernel trampoline virtual address, used for context switching and trap handling.
pub(crate) const TRAMPOLINE: usize = MAXVA - PGSIZE;

// Core memory layout constants for RISC-V QEMU virt machine.
pub(crate) const KERNBASE: usize = 0x8020_0000; // Base physical address of kernel image
const UART0: usize = 0x1000_0000; // MMIO address for UART console
const VIRTIO0: usize = 0x1000_1000; // MMIO address for VirtIO disk controller
const PLIC: usize = 0x0c00_0000; // Platform Level Interrupt Controller base
const PLIC_SIZE: usize = 0x0040_0000; // PLIC MMIO region size (4MB)
const MAXVA: usize = 1 << 38; // Sv39 maximum canonical VA bound (256 GB)

// Page Table Entry (PTE) Permission and Status Flags (RISC-V Privileged Architecture Spec).
const PTE_V: usize = 1 << 0; // Valid: indicates the entry is active and valid
pub(crate) const PTE_R: usize = 1 << 1; // Read permission
pub(crate) const PTE_W: usize = 1 << 2; // Write permission
pub(crate) const PTE_X: usize = 1 << 3; // Execute permission

unsafe extern "C" {
    /// Linker symbol marking the end of the kernel `.text` (code) section.
    static etext: u8;
}

/// Global atomic pointer to the root kernel page table, ensuring thread-safe boot initialization.
static KERNEL_PAGETABLE: AtomicUsize = AtomicUsize::new(0);

/// Extract the 9-bit page-table index for the given level from a virtual address.
///
/// In Sv39:
/// - Level 2 (VPN[2]): bits [38:30]
/// - Level 1 (VPN[1]): bits [29:21]
/// - Level 0 (VPN[0]): bits [20:12]
#[inline]
const fn px(level: usize, va: usize) -> usize {
    (va >> (12 + level * 9)) & 0x1ff
}

/// Calculate the virtual address for the kernel stack of process `i`.
///
/// In Sv39, kernel stacks are placed below TRAMPOLINE.
/// Each process kernel stack is separated by an unmapped guard page to catch overflows.
#[inline]
pub(crate) const fn kstack(i: usize) -> usize {
    TRAMPOLINE - (i + 1) * 2 * PGSIZE
}

/// Convert a page-table entry (PTE) to its corresponding physical address (PA).
///
/// Extracts PPN bits [53:10] from the PTE and left-shifts by 12 to form a 56-bit PA.
#[inline]
const fn pte2pa(pte: Pte) -> usize {
    (pte >> 10) << 12
}

/// Convert a physical address (PA) to a page-table entry (PTE) baseline value.
///
/// Right-shifts the 4096-byte aligned PA by 12 to obtain the PPN, then shifts into position [53:10].
#[inline]
const fn pa2pte(pa: usize) -> Pte {
    (pa >> 12) << 10
}

/// Walk the Sv39 page table to find the leaf Page Table Entry (PTE) corresponding to `va`.
///
/// Returns a mutable raw pointer to the Level-0 PTE.
/// If `alloc` is true, missing intermediate page table pages (Level 2 & Level 1) will be allocated dynamically.
pub(crate) fn walk(mut pagetable: *mut PageTable, va: usize, alloc: bool) -> Option<*mut Pte> {
    if va >= MAXVA {
        panic!("walk: va out of range {:#x}", va);
    }

    // Traverse page table levels downwards: Level 2 (root) -> Level 1 -> Level 0 (leaf)
    for level in [2, 1] {
        // Compute raw pointer to current level's PTE without creating temporary Rust references,
        // preventing Undefined Behavior (UB) on uninitialized memory regions.
        let index = px(level, va);
        let pte = unsafe { &mut (*pagetable)[index] as *mut Pte };
        let entry = unsafe { ptr::read(pte) };

        if entry & PTE_V != 0 {
            // Intermediate PTE exists and is valid; descend to next level page table.
            pagetable = pte2pa(entry) as *mut PageTable;
        } else if alloc {
            // Allocate a clean 4KB frame for the intermediate page table.
            let page = KMEM.lock().kalloc();
            if page.is_null() {
                return None; // Out of physical memory
            }
            unsafe {
                // Zero-out the newly allocated page table frame.
                ptr::write_bytes(page, 0, PGSIZE);
                // Mark current level entry valid and point it to the new next-level table.
                ptr::write(pte, pa2pte(page as usize) | PTE_V);
            }
            pagetable = page as *mut PageTable;
        } else {
            // PTE invalid and allocation not requested.
            return None;
        }
    }

    // Return raw pointer to the target Level 0 PTE.
    let index = px(0, va);
    Some(unsafe { &mut (*pagetable)[index] as *mut Pte })
}

/// Map a continuous virtual memory region `[va, va + size)` to physical addresses starting at `pa`.
///
/// Requires virtual address `va` and size `size` to be page-aligned (or rounded automatically).
/// Panics if memory allocation fails or an existing mapping is overwritten (remap attempt).
pub(crate) fn mappages(
    pagetable: *mut PageTable,
    va: usize,
    size: usize,
    mut pa: usize,
    perm: usize,
) {
    if size == 0 {
        return;
    }

    let mut a = page_round_down(va);
    let last = page_round_down(va + size - 1);
    pa = page_round_down(pa);

    loop {
        // Retrieve or allocate the Level 0 PTE for the current page address `a`.
        let pte = walk(pagetable, a, true).expect("mappages: out of memory");
        unsafe {
            if *pte & PTE_V != 0 {
                panic!("mappages: remap {:#x}", a);
            }
            // Populate leaf PTE with Physical Page Number (PPN), permissions, and Valid bit.
            *pte = pa2pte(pa) | perm | PTE_V;
        }
        if a == last {
            break;
        }
        a += PGSIZE;
        pa += PGSIZE;
    }
}

/// Create and initialize the root Sv39 kernel page table.
///
/// Sets up direct/identity mappings for kernel MMIO (UART, PLIC), executable code (`.text`),
/// and read/write kernel data/RAM.
pub(crate) fn kvmmake() -> *mut PageTable {
    // Allocate the root Level 2 page table frame.
    let pagetable = KMEM.lock().kalloc() as *mut PageTable;
    if pagetable.is_null() {
        panic!("kvmmake: out of memory");
    }
    unsafe { ptr::write_bytes(pagetable as *mut u8, 0, PGSIZE) };

    // 1. Map UART MMIO region (Read + Write)
    mappages(pagetable, UART0, PGSIZE, UART0, PTE_R | PTE_W);

    // 2. Map VirtIO MMIO region (Read + Write)
    mappages(pagetable, VIRTIO0, PGSIZE, VIRTIO0, PTE_R | PTE_W);

    // 3. Map PLIC MMIO region (Read + Write)
    mappages(pagetable, PLIC, PLIC_SIZE, PLIC, PTE_R | PTE_W);

    #[allow(unused_unsafe)]
    let text_end = unsafe { ptr::addr_of!(etext) as usize };

    // 4. Map Kernel Text section (Read + Execute)
    mappages(
        pagetable,
        KERNBASE,
        text_end - KERNBASE,
        KERNBASE,
        PTE_R | PTE_X,
    );

    // 5. Map Kernel Data and dynamic heap region up to PHYSTOP (Read + Write)
    mappages(
        pagetable,
        text_end,
        PHYSTOP - text_end,
        text_end,
        PTE_R | PTE_W,
    );

    // 6. Map kernel stacks for all processes (Read + Write)
    for i in 0..NPROC {
        let pa = KMEM.lock().kalloc();
        if pa.is_null() {
            panic!("kvmmake: kalloc failed for proc stack");
        }
        mappages(pagetable, kstack(i), PGSIZE, pa as usize, PTE_R | PTE_W);
    }

    pagetable
}

/// Initialize the global kernel page table state once during boot.
pub(crate) fn kvminit() {
    KERNEL_PAGETABLE.store(kvmmake() as usize, Ordering::Release);
}

/// Activate the kernel Sv39 page table on the executing CPU hart.
///
/// Writes the root page table physical address into the `satp` CSR and flushes TLB cache.
pub(crate) fn kvminithart() {
    let pagetable = KERNEL_PAGETABLE.load(Ordering::Acquire);
    if pagetable == 0 {
        panic!("kvminithart: no kernel page table");
    }

    // Flush Translation Lookaside Buffer (TLB) before updating page tables.
    csr::sfence_vma();

    // Configure satp register: Mode = 8 (Sv39), PPN = pagetable >> 12
    csr::write_satp((8usize << 60) | (pagetable >> 12));

    // Flush TLB after loading the new address space.
    csr::sfence_vma();
}

/// Translate a kernel virtual address (VA) to its corresponding physical address (PA).
///
/// Returns `None` if the virtual address is not mapped or invalid.
#[allow(unused)]
pub(crate) fn kernel_pa(va: usize) -> Option<usize> {
    let pagetable = KERNEL_PAGETABLE.load(Ordering::Acquire) as *mut PageTable;
    let pte = walk(pagetable, va, false)?;
    let entry = unsafe { *pte };

    // If valid, combine physical page base address with page offset (lower 12 bits of va).
    (entry & PTE_V != 0).then(|| pte2pa(entry) | (va & (PGSIZE - 1)))
}
