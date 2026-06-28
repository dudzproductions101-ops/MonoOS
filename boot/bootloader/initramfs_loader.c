/*
 * initramfs_loader.c – load the initial RAM filesystem into memory
 *
 * The initramfs (or initrd) is a compressed cpio archive that Linux
 * unpacks into tmpfs at startup.  The boot protocol requires us to:
 *
 *   boot_params->hdr.ramdisk_image  = low 32 bits of physical address
 *   boot_params->hdr.ramdisk_size   = byte size
 *   boot_params->ext_ramdisk_image  = high 32 bits (if above 4 GiB)
 *   boot_params->ext_ramdisk_size   = high 32 bits of size (usually 0)
 *
 * The initrd must be placed below initrd_addr_max (found in the kernel
 * setup header; usually 4 GiB for 32-bit kernels, effectively
 * unrestricted for 64-bit kernels compiled with HIGHMEM support).
 *
 * We load the image already placed by GRUB (as a multiboot2 module)
 * at INITRAMFS_LOAD_ADDR.  If the GRUB module base differs we copy it.
 */

#include "boot.h"
#include "memory.h"

#include <stdint.h>
#include <stddef.h>

/* Provided by kernel_loader.c (shared module globals) */
extern uint64_t g_initrd_module_base;
extern uint64_t g_initrd_module_size;

/* Simple memcpy without libc */
static void *memcpy_r(void *dst, const void *src, size_t n)
{
    uint8_t *d = (uint8_t *)dst;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
    return dst;
}

/* ------------------------------------------------------------------ */
/*  Detect initramfs format (gzip, xz, zstd, lz4, lzma) by magic     */
/* ------------------------------------------------------------------ */
typedef enum {
    INITRD_FMT_UNKNOWN = 0,
    INITRD_FMT_CPIO_NEWC,
    INITRD_FMT_CPIO_CRC,
    INITRD_FMT_GZIP,
    INITRD_FMT_XZ,
    INITRD_FMT_ZSTD,
    INITRD_FMT_LZ4,
    INITRD_FMT_LZMA,
} initrd_format_t;

static initrd_format_t detect_format(const uint8_t *data, size_t size)
{
    if (size < 6) return INITRD_FMT_UNKNOWN;

    /* cpio newc */
    if (data[0] == '0' && data[1] == '7' && data[2] == '0' &&
        data[3] == '7' && data[4] == '0' && data[5] == '1')
        return INITRD_FMT_CPIO_NEWC;

    /* cpio crc */
    if (data[0] == '0' && data[1] == '7' && data[2] == '0' &&
        data[3] == '7' && data[4] == '0' && data[5] == '2')
        return INITRD_FMT_CPIO_CRC;

    /* gzip */
    if (data[0] == 0x1F && data[1] == 0x8B)
        return INITRD_FMT_GZIP;

    /* xz */
    if (data[0] == 0xFD && data[1] == '7' && data[2] == 'z' &&
        data[3] == 'X' && data[4] == 'Z' && data[5] == 0x00)
        return INITRD_FMT_XZ;

    /* zstd */
    if (data[0] == 0x28 && data[1] == 0xB5 &&
        data[2] == 0x2F && data[3] == 0xFD)
        return INITRD_FMT_ZSTD;

    /* lz4 legacy */
    if (data[0] == 0x02 && data[1] == 0x21 &&
        data[2] == 0x4C && data[3] == 0x18)
        return INITRD_FMT_LZ4;

    /* lzma */
    if (data[0] == 0x5D && data[1] == 0x00)
        return INITRD_FMT_LZMA;

    return INITRD_FMT_UNKNOWN;
}

static const char *format_name(initrd_format_t fmt)
{
    switch (fmt) {
    case INITRD_FMT_CPIO_NEWC: return "cpio(newc)";
    case INITRD_FMT_CPIO_CRC:  return "cpio(crc)";
    case INITRD_FMT_GZIP:      return "gzip";
    case INITRD_FMT_XZ:        return "xz";
    case INITRD_FMT_ZSTD:      return "zstd";
    case INITRD_FMT_LZ4:       return "lz4";
    case INITRD_FMT_LZMA:      return "lzma";
    default:                   return "unknown";
    }
}

/* ------------------------------------------------------------------ */
/*  initramfs_loader_load                                              */
/* ------------------------------------------------------------------ */
boot_status_t initramfs_loader_load(const char *path, boot_context_t *ctx)
{
    (void)path;

    uint8_t *img  = (uint8_t *)(uintptr_t)g_initrd_module_base;
    uint64_t size = g_initrd_module_size;

    if (size == 0) {
        /*
         * No initramfs provided.  This is valid in some configurations
         * (e.g. root on NFS or built-in initramfs in kernel).
         * We proceed without setting the ramdisk fields.
         */
        serial_puts("[initramfs] No initramfs module loaded (size=0), "
                    "skipping\n");
        ctx->initramfs.load_addr = NULL;
        ctx->initramfs.size      = 0;
        return BOOT_STATUS_OK;
    }

    /* Detect format for informational purposes */
    initrd_format_t fmt = detect_format(img, (size_t)size);
    serial_puts("[initramfs] Detected format: ");
    serial_puts(format_name(fmt));
    serial_puts("\n");

    /* Check initrd_addr_max from the kernel setup header */
    struct boot_params *bp = ctx->boot_params;
    uint64_t addr_max = (uint64_t)bp->hdr.initrd_addr_max;
    if (addr_max == 0) {
        /* Old kernel with no field – default to 4 GiB */
        addr_max = 0xFFFFFFFFULL;
    }

    /* Determine where to place the initramfs */
    uint64_t dst_phys = (uint64_t)(uintptr_t)img;

    /* If GRUB placed it above the kernel but below addr_max, keep it */
    if (dst_phys < KERNEL_LOAD_ADDR || dst_phys + size > addr_max) {
        /* Need to move it to our default location */
        dst_phys = INITRAMFS_LOAD_ADDR;

        if (dst_phys + size > addr_max) {
            serial_puts("[initramfs] Error: initramfs + addr_max overflow\n");
            return BOOT_STATUS_NO_MEMORY;
        }

        serial_puts("[initramfs] Relocating initramfs to ");
        serial_puthex64(dst_phys);
        serial_puts("\n");

        void *dst = (void *)(uintptr_t)dst_phys;
        memcpy_r(dst, img, (size_t)size);
    }

    /* Verify we don't overlap the kernel */
    if (dst_phys < KERNEL_LOAD_ADDR + ctx->kernel.size &&
        dst_phys + size > KERNEL_LOAD_ADDR) {
        serial_puts("[initramfs] Error: initramfs overlaps kernel\n");
        return BOOT_STATUS_ERROR;
    }

    /* Write into boot_params */
    bp->hdr.ramdisk_image   = (uint32_t)(dst_phys & 0xFFFFFFFFULL);
    bp->hdr.ramdisk_size    = (uint32_t)(size & 0xFFFFFFFFULL);
    bp->ext_ramdisk_image   = (uint32_t)(dst_phys >> 32);
    bp->ext_ramdisk_size    = (uint32_t)(size >> 32);

    /* Fill context */
    ctx->initramfs.load_addr = (void *)(uintptr_t)dst_phys;
    ctx->initramfs.size      = size;

    serial_puts("[initramfs] Loaded at phys=");
    serial_puthex64(dst_phys);
    serial_puts(" size=");
    serial_puthex64(size);
    serial_puts("\n");

    return BOOT_STATUS_OK;
}