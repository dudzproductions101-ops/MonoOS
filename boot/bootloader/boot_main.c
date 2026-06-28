/*
 * boot_main.c – MonoOS bootloader entry point
 *
 * Called by GRUB after it loads our bootloader stub.  We are running as
 * a 32-bit / 64-bit PE binary inside the GRUB EFI environment.
 *
 * Responsibilities:
 *   1. Initialise the serial debug port.
 *   2. Discover UEFI memory map (passed by GRUB via multiboot2 tag or
 *      EFI handover).
 *   3. Reserve physical pages for boot_params, cmdline, kernel, initramfs.
 *   4. Drive the full boot sequence:
 *        a. Load kernel image.
 *        b. Load initramfs.
 *        c. Call Rust secure-boot verification layer via FFI.
 *        d. Build kernel command line.
 *        e. Hand off to Linux.
 *
 * Design note: we deliberately avoid libc.  All helpers are inline or
 * defined in companion translation units.
 */

#include "boot.h"
#include "memory.h"

#include <stdint.h>
#include <stddef.h>

/* ------------------------------------------------------------------ */
/*  Forward declarations for Rust FFI functions                        */
/*  The Rust secure_boot crate exposes these as extern "C" symbols.   */
/* ------------------------------------------------------------------ */
extern int monoos_verify_kernel(const void *image, uint64_t size);
extern int monoos_verify_initramfs(const void *image, uint64_t size);

/* ------------------------------------------------------------------ */
/*  Minimal string helpers (no libc)                                   */
/* ------------------------------------------------------------------ */
static size_t strlen_s(const char *s)
{
    size_t n = 0;
    while (s && s[n]) n++;
    return n;
}

static void *memset_s(void *dst, int c, size_t n)
{
    uint8_t *p = (uint8_t *)dst;
    while (n--) *p++ = (uint8_t)c;
    return dst;
}

static void *memcpy_s(void *dst, const void *src, size_t n)
{
    uint8_t *d = (uint8_t *)dst;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
    return dst;
}

/* ------------------------------------------------------------------ */
/*  Global boot context (statically allocated – no heap needed here)  */
/* ------------------------------------------------------------------ */
static boot_context_t g_boot_ctx;
static struct boot_params g_boot_params;

/* ------------------------------------------------------------------ */
/*  Serial port helpers (polled 16550 UART on COM1, 0x3F8)            */
/* ------------------------------------------------------------------ */
#define UART_BASE 0x3F8

static inline void outb(uint16_t port, uint8_t val)
{
    __asm__ volatile ("outb %0, %1" : : "a"(val), "Nd"(port));
}

static inline uint8_t inb(uint16_t port)
{
    uint8_t val;
    __asm__ volatile ("inb %1, %0" : "=a"(val) : "Nd"(port));
    return val;
}

void serial_init(void)
{
    outb(UART_BASE + 1, 0x00); /* Disable interrupts                  */
    outb(UART_BASE + 3, 0x80); /* Enable DLAB (set baud rate divisor) */
    outb(UART_BASE + 0, 0x03); /* Divisor lo: 38400 baud              */
    outb(UART_BASE + 1, 0x00); /* Divisor hi                          */
    outb(UART_BASE + 3, 0x03); /* 8 bits, no parity, one stop bit     */
    outb(UART_BASE + 2, 0xC7); /* Enable FIFO, clear, 14-byte thresh  */
    outb(UART_BASE + 4, 0x0B); /* IRQs enabled, RTS/DSR set           */
}

static void serial_putc(char c)
{
    while ((inb(UART_BASE + 5) & 0x20) == 0) { /* wait for THR empty  */ }
    outb(UART_BASE, (uint8_t)c);
}

void serial_puts(const char *s)
{
    while (s && *s) {
        if (*s == '\n') serial_putc('\r');
        serial_putc(*s++);
    }
}

void serial_puthex64(uint64_t v)
{
    static const char hex[] = "0123456789abcdef";
    serial_puts("0x");
    for (int i = 60; i >= 0; i -= 4)
        serial_putc(hex[(v >> i) & 0xF]);
}

/* ------------------------------------------------------------------ */
/*  Panic – print message and halt                                     */
/* ------------------------------------------------------------------ */
void __attribute__((noreturn)) boot_panic(const char *msg)
{
    serial_puts("\n[BOOT PANIC] ");
    serial_puts(msg);
    serial_puts("\n");
    /* Halt */
    for (;;) __asm__ volatile ("cli; hlt");
}

/* ------------------------------------------------------------------ */
/*  EFI memory map discovery                                           */
/*                                                                     */
/*  GRUB 2 multiboot2 tags carry either a Multiboot2 memory map or    */
/*  the EFI memory map (tag type 17).  We accept a simplified handoff */
/*  structure passed in register RDI (System V AMD64 ABI, first arg). */
/*                                                                     */
/*  In a real integration GRUB passes a *multiboot2_info* pointer in  */
/*  RBX.  We model that here.                                          */
/* ------------------------------------------------------------------ */

/* Multiboot2 header magic and tag types we care about */
#define MB2_MAGIC           0x36D76289
#define MB2_TAG_END         0
#define MB2_TAG_MEM_MAP     6
#define MB2_TAG_EFI_MMAP    17
#define MB2_TAG_EFI_ST64    12

typedef struct {
    uint32_t type;
    uint32_t size;
} mb2_tag_header_t;

typedef struct {
    uint32_t type;
    uint32_t size;
    uint32_t entry_size;
    uint32_t entry_version;
    /* followed by efi_memory_descriptor_t[] */
} mb2_tag_efi_mmap_t;

typedef struct {
    uint32_t total_size;
    uint32_t reserved;
    /* followed by tags */
} mb2_info_t;

static boot_status_t parse_multiboot2(uint64_t mb2_phys, boot_context_t *ctx)
{
    mb2_info_t *info = (mb2_info_t *)(uintptr_t)mb2_phys;
    uint8_t *tags = (uint8_t *)(info + 1);
    uint8_t *end  = (uint8_t *)info + info->total_size;

    mem_map_t *mmap = (mem_map_t *)heap_alloc(NULL, sizeof(mem_map_t), 8);
    if (!mmap) return BOOT_STATUS_NO_MEMORY;
    memset_s(mmap, 0, sizeof(mem_map_t));

    while (tags < end) {
        mb2_tag_header_t *hdr = (mb2_tag_header_t *)tags;
        if (hdr->type == MB2_TAG_END) break;

        if (hdr->type == MB2_TAG_EFI_MMAP) {
            mb2_tag_efi_mmap_t *emmap = (mb2_tag_efi_mmap_t *)hdr;
            uint8_t *desc_ptr  = (uint8_t *)(emmap + 1);
            uint8_t *desc_end  = (uint8_t *)hdr + hdr->size;
            size_t n_desc = 0;

            /* Count descriptors */
            for (uint8_t *p = desc_ptr; p + emmap->entry_size <= desc_end;
                 p += emmap->entry_size)
                n_desc++;

            mem_map_parse_efi((efi_memory_descriptor_t *)desc_ptr,
                               n_desc, emmap->entry_size, mmap);

            /* Populate E820 table in boot_params */
            mem_map_to_e820(mmap,
                             ctx->boot_params->e820_table,
                             &ctx->boot_params->e820_entries);

            serial_puts("[boot] EFI memory map parsed, entries: ");
            serial_puthex64(ctx->boot_params->e820_entries);
            serial_puts("\n");
            return BOOT_STATUS_OK;
        }

        /* Align to next 8-byte boundary */
        tags += (hdr->size + 7) & ~7U;
    }

    serial_puts("[boot] Warning: no EFI mmap tag found, using dummy map\n");
    /* Fallback: single RAM region 0-256 MiB (for emulator testing) */
    ctx->boot_params->e820_table[0].addr = 0x100000;
    ctx->boot_params->e820_table[0].size = 0x0F000000;
    ctx->boot_params->e820_table[0].type = 1; /* RAM */
    ctx->boot_params->e820_entries = 1;
    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  boot_main_init – called from the assembly trampoline               */
/* ------------------------------------------------------------------ */
boot_status_t boot_main_init(boot_context_t *ctx)
{
    serial_puts("[boot] MonoOS boot stage starting\n");

    memset_s(&g_boot_params, 0, sizeof(g_boot_params));
    ctx->boot_params = &g_boot_params;
    ctx->secure_boot_verified = 0;
    ctx->mode = BOOT_MODE_NORMAL;

    /* Initialise CPU (ensure we are in a clean long-mode state)       */
    cpu_init_early();
    serial_puts("[boot] CPU init done\n");

    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  boot_entry – actual C entry point called from entry.S              */
/*  Arguments follow System V AMD64 ABI:                               */
/*    rdi = multiboot2 magic (should be MB2_MAGIC)                     */
/*    rsi = physical address of multiboot2 info structure              */
/* ------------------------------------------------------------------ */
void __attribute__((section(".boot.entry")))
boot_entry(uint64_t mb2_magic, uint64_t mb2_info_phys)
{
    serial_init();
    serial_puts("[boot] boot_entry reached\n");

    if (mb2_magic != MB2_MAGIC) {
        boot_panic("Multiboot2 magic mismatch – not loaded by GRUB2?");
    }

    boot_status_t st;

    /* Zero out the global context */
    memset_s(&g_boot_ctx, 0, sizeof(g_boot_ctx));

    /* Basic init */
    st = boot_main_init(&g_boot_ctx);
    if (st != BOOT_STATUS_OK) boot_panic("boot_main_init failed");

    /* Parse multiboot2 info to build E820 map */
    st = parse_multiboot2(mb2_info_phys, &g_boot_ctx);
    if (st != BOOT_STATUS_OK) boot_panic("memory map init failed");

    /* Determine boot mode from persistent flags */
    /* (boot_flags is in the Rust layer; for C we default to NORMAL)   */
    g_boot_ctx.mode = BOOT_MODE_NORMAL;

    /* Build default command line */
    st = boot_args_build(&g_boot_ctx,
                          "root=/dev/sda2 rw quiet loglevel=3 "
                          "init=/lib/systemd/systemd");
    if (st != BOOT_STATUS_OK) boot_panic("boot_args_build failed");

    /* Load kernel image */
    serial_puts("[boot] Loading kernel\n");
    st = kernel_loader_load("/boot/vmlinuz", &g_boot_ctx);
    if (st != BOOT_STATUS_OK) boot_panic("kernel_loader_load failed");

    /* Load initramfs */
    serial_puts("[boot] Loading initramfs\n");
    st = initramfs_loader_load("/boot/initramfs.img", &g_boot_ctx);
    if (st != BOOT_STATUS_OK) boot_panic("initramfs_loader_load failed");

    /* Secure boot verification via Rust FFI */
    serial_puts("[boot] Verifying signatures\n");
    int kv = monoos_verify_kernel(g_boot_ctx.kernel.load_addr,
                                  g_boot_ctx.kernel.size);
    if (kv != 0) boot_panic("Kernel signature verification FAILED");

    int iv = monoos_verify_initramfs(g_boot_ctx.initramfs.load_addr,
                                     g_boot_ctx.initramfs.size);
    if (iv != 0) boot_panic("Initramfs signature verification FAILED");

    g_boot_ctx.secure_boot_verified = 1;
    g_boot_ctx.boot_params->secure_boot = 1;
    serial_puts("[boot] Secure boot: OK\n");

    /* Hand off to Linux */
    serial_puts("[boot] Handing off to Linux kernel\n");
    kernel_loader_handoff(&g_boot_ctx);

    /* Should never reach here */
    boot_panic("kernel handoff returned unexpectedly");
}