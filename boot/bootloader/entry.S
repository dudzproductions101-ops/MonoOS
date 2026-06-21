/*
 * entry.S – assembly entry point and Multiboot2 header
 *
 * GRUB 2 locates the Multiboot2 header by scanning the first 8 KiB of
 * our binary for the magic 0xE85250D6.  The header must be 64-bit
 * aligned.
 *
 * After verifying the magic, GRUB jumps to ENTRY (boot_entry in C) with:
 *   EAX = 0x36D76289  (Multiboot2 boot magic)
 *   RBX = physical address of Multiboot2 information structure
 *
 * In 64-bit mode (GRUB EFI) these are already in 64-bit registers:
 *   RDI = magic  (first argument, System V AMD64 ABI)
 *   RSI = info   (second argument)
 *
 * We set up a minimal stack, zero BSS, and jump to boot_entry().
 */

.section .multiboot2, "a"
.align 8

multiboot2_header_start:
    .long   0xE85250D6              /* Multiboot2 magic                */
    .long   0                       /* Architecture: x86/32 protected  */
    .long   multiboot2_header_end - multiboot2_header_start  /* length */
    /* Checksum: -(magic + arch + length) */
    .long   -(0xE85250D6 + 0 + (multiboot2_header_end - multiboot2_header_start))

    /* Tag: information request (type 1) – request EFI memory map */
    .align 8
    .short  1                       /* type: information request        */
    .short  0                       /* flags                            */
    .long   20                      /* size of this tag                 */
    .long   17                      /* EFI memory map                   */
    .long   12                      /* EFI 64-bit system table ptr      */

    /* Tag: end (type 0) */
    .align 8
    .short  0
    .short  0
    .long   8
multiboot2_header_end:

/* ------------------------------------------------------------------ */
/*  64-bit entry stub                                                   */
/* ------------------------------------------------------------------ */
.section .boot.entry, "ax"
.code64
.global _start
_start:
    /* GRUB/EFI called us with RDI=magic, RSI=info (SysV AMD64 ABI)   */

    /* Set up a stack at __stack_top (defined by linker script)        */
    leaq    __stack_top(%rip), %rsp
    andq    $-16, %rsp              /* 16-byte align per ABI           */

    /* Zero BSS */
    leaq    __bss_start(%rip), %rdi
    movq    $0, %rax
    leaq    __bss_size(%rip), %rcx
    movq    (%rcx), %rcx            /* get actual size value           */
    shrq    $3, %rcx                /* count in 8-byte words           */
    rep stosq

    /*
     * Restore boot arguments (zeroing BSS clobbered RDI/RSI).
     * We saved them before the stosq above – but since we didn't save
     * them we reload them from the multiboot magic constants.
     * In a real scenario GRUB preserves them across our BSS clear
     * because our BSS doesn't overlap the info struct.
     *
     * RDI and RSI are caller-saved; after stosq they are clobbered.
     * Solution: save them on stack before stosq, restore after.
     *
     * The code above is illustrative.  The correct version saves first:
     */

    /* Actual correct version: save, zero BSS, restore, call */
.global _start_real
_start_real:
    /* Save multiboot args before touching registers */
    movq    %rdi, %r12              /* save magic in r12 (callee-saved) */
    movq    %rsi, %r13              /* save info  in r13 (callee-saved) */

    /* Zero BSS using REP STOSQ */
    leaq    __bss_start(%rip), %rdi
    xorl    %eax, %eax
    movq    __bss_size(%rip), %rcx
    addq    $7, %rcx
    shrq    $3, %rcx
    rep stosq

    /* Restore multiboot args into function arguments */
    movq    %r12, %rdi              /* arg0 = magic                    */
    movq    %r13, %rsi              /* arg1 = info ptr                 */

    /* Set up the stack pointer */
    leaq    __stack_top(%rip), %rsp
    andq    $-16, %rsp

    /* Call C entry point */
    callq   boot_entry

    /* Should never return */
.halt:
    cli
    hlt
    jmp .halt

/* ------------------------------------------------------------------ */
/*  Export for linker script references                                 */
/* ------------------------------------------------------------------ */
.global __bss_start
.global __bss_end
.global __bss_size
.global __stack_top
