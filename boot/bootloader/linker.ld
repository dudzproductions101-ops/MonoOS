/*
 * linker.ld – OneOS bootloader linker script
 *
 * Produces a flat ELF64 binary loaded by GRUB as a multiboot2 module.
 * The multiboot2 header must appear within the first 8 KiB of the file.
 *
 * Memory layout (all physical addresses):
 *   0x100000  – our bootloader binary is loaded here by GRUB
 *   0x10000   – boot_params scratch (zero page)
 *   0x20000   – kernel cmdline buffer
 *   0x200000  – kernel protected-mode image
 *   0x4000000 – initramfs image
 */

OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(boot_entry)

SECTIONS
{
    /* GRUB loads us at 1 MiB */
    . = 0x100000;

    /*
     * Multiboot2 header section – must be within first 8192 bytes and
     * 64-bit aligned.  Placed first so GRUB finds it.
     */
    .multiboot2 : ALIGN(8) {
        KEEP(*(.multiboot2))
    }

    /* Boot entry code (section attribute ".boot.entry") */
    .boot.entry : ALIGN(16) {
        KEEP(*(.boot.entry))
    }

    /* All other code */
    .text : ALIGN(16) {
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata : ALIGN(16) {
        *(.rodata .rodata.*)
    }

    /* Initialised data */
    .data : ALIGN(16) {
        *(.data .data.*)
    }

    /* BSS – zero-initialised */
    .bss : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Stack – 64 KiB, 16-byte aligned (ABI requirement) */
    .stack : ALIGN(4096) {
        . += 0x10000;
        __stack_top = .;
    }

    /* Discard DWARF / debug info that inflates size */
    /DISCARD/ : {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
        *(.gcc_except_table*)
    }
}

/*
 * Symbols used by the C startup shim to zero BSS and set the stack.
 * (These are referenced by the assembly entry stub in entry.S if used.)
 */
__bss_size   = __bss_end - __bss_start;
__load_start = 0x100000;
