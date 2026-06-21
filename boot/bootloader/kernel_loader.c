/*
 * kernel_loader.c – load a Linux bzImage and hand off via boot protocol
 *
 * The Linux x86-64 boot protocol (v2.12) requires:
 *   1. Copy the kernel setup sector(s) to a buffer.
 *   2. Populate boot_params (the "zero page") fully.
 *   3. Set the kernel cmdline pointer.
 *   4. Set the initrd address + size.
 *   5. Jump to 64-bit kernel entry point: kernel_base + 0x200.
 *
 * The kernel image (bzImage) layout:
 *   [0x000 .. 0x1F0]  Real-mode boot sector (ignored in 64-bit)
 *   [0x1F1]            setup_header begins
 *   [0x1FE]            Boot flag (0xAA55)
 *   [setup_sects * 512 + 512] .. end   Protected-mode kernel code
 *
 * Reference: Linux Documentation/x86/boot.rst
 */

#include "boot.h"
#include "memory.h"

#include <stdint.h>
#include <stddef.h>

/* ------------------------------------------------------------------ */
/*  Internal helpers                                                    */
/* ------------------------------------------------------------------ */
static void *memcpy_k(void *dst, const void *src, size_t n)
{
    uint8_t *d = (uint8_t *)dst;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
    return dst;
}

static void *memset_k(void *dst, int c, size_t n)
{
    uint8_t *p = (uint8_t *)dst;
    while (n--) *p++ = (uint8_t)c;
    return dst;
}

/* ------------------------------------------------------------------ */
/*  Simulated file-read shim                                           */
/*                                                                     */
/*  In a real GRUB integration we would call GRUB's grub_file_open /  */
/*  grub_file_read APIs (via extern declarations) or use the EFI       */
/*  Simple File System Protocol.  Here we implement a thin shim that  */
/*  reads from a fixed physical address pre-loaded by GRUB's          */
/*  multiboot2 module tag.  The module start/end are communicated via  */
/*  module_phys_base / module_phys_size which GRUB populates in the   */
/*  multiboot2 modules tag (type 3).                                   */
/*                                                                     */
/*  For integration with real GRUB: replace boot_file_read() with     */
/*  calls to grub_file_read().                                         */
/* ------------------------------------------------------------------ */

/*
 * These two symbols are expected to be filled in by the multiboot2
 * module parsing code (boot_main.c) before kernel_loader_load is
 * called.  They are extern so the linker resolves them globally.
 */
extern uint64_t g_kernel_module_base;   /* physical address of kernel  */
extern uint64_t g_kernel_module_size;   /* byte size of kernel image   */
extern uint64_t g_initrd_module_base;
extern uint64_t g_initrd_module_size;

/* Provide default values so the file compiles standalone */
uint64_t g_kernel_module_base = KERNEL_LOAD_ADDR;
uint64_t g_kernel_module_size = 0;
uint64_t g_initrd_module_base = INITRAMFS_LOAD_ADDR;
uint64_t g_initrd_module_size = 0;

/* ------------------------------------------------------------------ */
/*  Validate Linux bzImage magic fields                                */
/* ------------------------------------------------------------------ */
static boot_status_t validate_bzimage(const uint8_t *img, size_t size)
{
    if (size < 0x210) {
        serial_puts("[kernel] Image too small to contain setup header\n");
        return BOOT_STATUS_BAD_MAGIC;
    }

    const struct setup_header *hdr =
        (const struct setup_header *)(img + SETUP_HEADER_OFFSET);

    if (hdr->boot_flag != BOOT_FLAG_MAGIC) {
        serial_puts("[kernel] Boot flag mismatch (expected 0xAA55)\n");
        serial_puthex64(hdr->boot_flag);
        serial_puts("\n");
        return BOOT_STATUS_BAD_MAGIC;
    }

    if (hdr->header != LINUX_MAGIC) {
        serial_puts("[kernel] HdrS magic mismatch\n");
        return BOOT_STATUS_BAD_MAGIC;
    }

    uint16_t ver = hdr->version;
    if (ver < 0x020c) {
        serial_puts("[kernel] Boot protocol < 2.12 not supported\n");
        return BOOT_STATUS_BAD_MAGIC;
    }

    serial_puts("[kernel] bzImage validated, protocol version: ");
    serial_puthex64(ver);
    serial_puts("\n");
    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  kernel_loader_load                                                 */
/*                                                                     */
/*  Copies the kernel already loaded by GRUB from its module region   */
/*  to KERNEL_LOAD_ADDR and populates the boot context.               */
/* ------------------------------------------------------------------ */
boot_status_t kernel_loader_load(const char *path, boot_context_t *ctx)
{
    (void)path; /* Path used by GRUB module – already mapped */

    uint8_t *img      = (uint8_t *)(uintptr_t)g_kernel_module_base;
    uint64_t img_size = g_kernel_module_size;

    if (img_size == 0) {
        /*
         * Fallback for emulator / testing: pretend we have a minimal
         * kernel at the module base address.  In production this path
         * should not be reached.
         */
        serial_puts("[kernel] Warning: kernel module size is 0, "
                    "using placeholder\n");
        img_size = 0x1000; /* 4 KiB placeholder */
    }

    boot_status_t st = validate_bzimage(img, img_size);
    if (st != BOOT_STATUS_OK) return st;

    const struct setup_header *src_hdr =
        (const struct setup_header *)(img + SETUP_HEADER_OFFSET);

    /* Number of setup sectors (each 512 bytes) */
    uint32_t setup_sects = src_hdr->setup_sects;
    if (setup_sects == 0) setup_sects = 4; /* old kernels default to 4 */

    /* Byte size of the setup block (includes the initial 512 boot sector) */
    uint64_t setup_size = ((uint64_t)setup_sects + 1) * 512ULL;

    /* The 32/64-bit protected-mode code follows immediately */
    uint64_t pm_offset = setup_size;
    uint64_t pm_size   = img_size - pm_offset;

    serial_puts("[kernel] setup_sects=");
    serial_puthex64(setup_sects);
    serial_puts(" setup_size=");
    serial_puthex64(setup_size);
    serial_puts(" pm_size=");
    serial_puthex64(pm_size);
    serial_puts("\n");

    /* Destination for protected-mode kernel code */
    uint8_t *dst = (uint8_t *)(uintptr_t)KERNEL_LOAD_ADDR;

    /* Zero destination first */
    memset_k(dst, 0, pm_size);

    /* Copy protected-mode image */
    memcpy_k(dst, img + pm_offset, pm_size);

    /* ------------------------------------------------------------- */
    /*  Populate boot_params setup header                             */
    /* ------------------------------------------------------------- */
    struct boot_params *bp = ctx->boot_params;

    /* Copy the entire setup header from the kernel image */
    memcpy_k(&bp->hdr, src_hdr, sizeof(struct setup_header));

    /* Override fields we must set */
    bp->hdr.type_of_loader   = BOOTLOADER_ID;
    bp->hdr.loadflags       |= 0x01;  /* LOADED_HIGH: kernel at 0x100000+ */
    bp->hdr.code32_start     = KERNEL_LOAD_ADDR;

    /* vid_mode: VGA_ASK = 0xFFFF (ask user), 0x0F00 = 80x25 text     */
    bp->hdr.vid_mode         = 0x0F00;

    /* ------------------------------------------------------------- */
    /*  Fill in context                                               */
    /* ------------------------------------------------------------- */
    ctx->kernel.load_addr  = dst;
    ctx->kernel.size       = pm_size;
    ctx->kernel.entry_point = KERNEL_LOAD_ADDR + 0x200; /* 64-bit entry */
    ctx->kernel.setup_sects = setup_sects;

    serial_puts("[kernel] Kernel loaded at: ");
    serial_puthex64((uint64_t)(uintptr_t)dst);
    serial_puts(" entry: ");
    serial_puthex64(ctx->kernel.entry_point);
    serial_puts("\n");

    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  kernel_loader_handoff                                              */
/*                                                                     */
/*  Hand control to the Linux kernel using the x86-64 boot protocol.  */
/*                                                                     */
/*  Calling convention mandated by Linux boot protocol:               */
/*    RSI = physical address of boot_params (zero page)               */
/*    All other registers undefined (kernel will set them up)         */
/*    CS:RIP = 64-bit kernel entry (kernel_base + 0x200)             */
/*    CR0 = protected mode enabled (bit 0)                            */
/*    CR4 = PAE enabled (bit 5)                                       */
/*    Paging: must be enabled if kernel was compiled with             */
/*            CONFIG_RELOCATABLE (it usually is)                      */
/*    GDT: flat 64-bit descriptor set by cpu_init_early()             */
/*                                                                     */
/*  We jump with a far jump to ensure we are in the correct code      */
/*  segment.  GRUB already put us in 64-bit long mode, so we just    */
/*  do a direct indirect call.                                         */
/* ------------------------------------------------------------------ */
boot_status_t __attribute__((noreturn))
kernel_loader_handoff(boot_context_t *ctx)
{
    uint64_t bp_phys     = (uint64_t)(uintptr_t)ctx->boot_params;
    uint64_t entry_phys  = ctx->kernel.entry_point;

    serial_puts("[kernel] Jumping to Linux at ");
    serial_puthex64(entry_phys);
    serial_puts(" with boot_params at ");
    serial_puthex64(bp_phys);
    serial_puts("\n");

    /*
     * Linux 64-bit boot entry requirements:
     *   - RSI must hold the physical address of boot_params
     *   - Interrupts disabled
     *   - Stack pointer valid (GRUB already set up a stack)
     *
     * We use an inline asm trampoline that:
     *   1. Disables interrupts.
     *   2. Loads RSI with boot_params physical address.
     *   3. Performs an indirect jmp to the kernel entry point.
     */
    __asm__ volatile (
        "cli\n\t"
        "movq %[bp], %%rsi\n\t"
        "jmp *%[entry]\n\t"
        :
        : [bp]    "r" (bp_phys),
          [entry] "r" (entry_phys)
        : "rsi", "memory"
    );

    /* Unreachable */
    __builtin_unreachable();
}