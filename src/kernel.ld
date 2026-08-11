/*
 * Generate a 64-bit little-endian RISC-V ELF binary.
 */
OUTPUT_FORMAT("elf64-littleriscv")

/*
 * Define the program entry point.
 * The CPU starts execution from the _start symbol.
 */
ENTRY(_start)

/*
 * Kernel load address for the QEMU RISC-V virt machine.
 * OpenSBI occupies the lower memory region, so the kernel
 * is loaded at 0x80200000.
 */
BASE_ADDRESS = 0x80200000;


SECTIONS
{
    /*
     * Set the starting address of the kernel image.
     */
    . = BASE_ADDRESS;
    skernel = .;


    /*
     * Text section.
     *
     * Keep the startup assembly code (.text.entry) at the
     * beginning of the kernel image, followed by Rust code.
     */
    .text : {
        KEEP(*(.text.entry))
        *(.text .text.*)
    }


    /*
     * Read-only data section.
     *
     * Contains string literals, constant data, and other
     * immutable objects.
     */
    .rodata : {
        *(.rodata .rodata.*)
    }


    /*
     * Initialized data section.
     *
     * Contains global and static variables with initial values.
     */
    .data : {
        *(.data .data.*)
    }


    /*
     * BSS section.
     *
     * Contains zero-initialized global/static variables and
     * the boot stack space.
     */
    .bss : {
        *(.bss .bss.*)
    }


    /*
     * Align the kernel end address to a 4KiB boundary.
     */
    . = ALIGN(4096);

    /*
     * Mark the end address of the kernel image.
     */
    ekernel = .;


    /*
     * Discard unnecessary unwind information.
     *
     * Bare-metal Rust does not require exception unwinding.
     */
    /DISCARD/ : {
        *(.eh_frame)
    }
}