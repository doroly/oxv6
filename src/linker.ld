/* Linker script for oxv6 RISC-V 64 kernel. */

/* Specify the output executable format as 64-bit Little-Endian RISC-V ELF. */
OUTPUT_FORMAT("elf64-littleriscv")

/* Specify the target machine architecture. */
OUTPUT_ARCH(riscv)

/* Define the kernel entry point symbol (implemented in assembly entry code). */
ENTRY(_start)

/* Define the base physical address where the kernel will be loaded.
 * 0x80200000 is the standard entry address jumped to by OpenSBI on QEMU virt. */
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    /* Set the location counter (.) to the kernel base address. */
    . = BASE_ADDRESS;

    /* Export symbol marking the start address of the kernel image. */
    skernel = .;

    /* Executable code section, 4-byte aligned for RISC-V instruction boundary. */
    .text : ALIGN(4)
    {
        /* Ensure the assembly entry point block is placed at the very beginning. */
        *(.text.entry)
        /* Include all other executable instructions. */
        *(.text .text.*)
    }

    /* Read-only data section (e.g., string literals, constants). */
    .rodata : ALIGN(4)
    {
        *(.rodata .rodata.*)
    }

    /* Initialized global and static variables. */
    .data : ALIGN(4)
    {
        *(.data .data.*)
    }

    /* Uninitialized global and static variables (zero-initialized at boot). */
    .bss : ALIGN(4)
    {
        *(COMMON)
        *(.bss .bss.*)
    }

    /* Align the location counter to a 4 KiB (4096-byte) page boundary.
     * Ensures the kernel end boundary is cleanly page-aligned for the page allocator. */
    . = ALIGN(4096);

    /* Export symbol marking the end address of the kernel image. */
    ekernel = .;

    /* Discard unneeded sections (e.g., C++ / Rust exception unwinding tables)
     * to keep the kernel binary clean and compact for bare-metal boot. */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
    }
}