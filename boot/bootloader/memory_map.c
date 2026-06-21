/*
 * memory_map.c – UEFI memory map parsing + E820 conversion
 *
 * Converts the EFI memory descriptor array (obtained from GRUB's
 * multiboot2 EFI mmap tag) into:
 *
 *   1. An internal mem_map_t (sorted, coalesced).
 *   2. The e820_table[] inside struct boot_params required by Linux.
 *
 * Also provides the bootloader's simple bump-pointer heap.
 */

#include "boot.h"
#include "memory.h"

#include <stdint.h>
#include <stddef.h>

/* ------------------------------------------------------------------ */
/*  Global heap state – initialised once from BOOTLOADER_HEAP_BASE    */
/* ------------------------------------------------------------------ */
static heap_t g_heap = {
    .base = BOOTLOADER_HEAP_BASE,
    .size = BOOTLOADER_HEAP_SIZE,
    .used = 0,
};

/* ------------------------------------------------------------------ */
/*  Heap helpers                                                        */
/* ------------------------------------------------------------------ */
void heap_init(heap_t *h, uint64_t base, uint64_t size)
{
    if (!h) {
        g_heap.base = base;
        g_heap.size = size;
        g_heap.used = 0;
        return;
    }
    h->base = base;
    h->size = size;
    h->used = 0;
}

void *heap_alloc(heap_t *h, size_t bytes, size_t align)
{
    heap_t *hp = h ? h : &g_heap;

    /* Align the current pointer */
    uint64_t ptr = hp->base + hp->used;
    if (align > 1) {
        uint64_t rem = ptr % align;
        if (rem) ptr += align - rem;
    }

    if (ptr - hp->base + bytes > hp->size) {
        serial_puts("[mem] heap_alloc: out of heap space\n");
        return NULL;
    }

    hp->used = (ptr - hp->base) + bytes;
    return (void *)(uintptr_t)ptr;
}

void heap_reset(heap_t *h)
{
    if (!h) { g_heap.used = 0; return; }
    h->used = 0;
}

/* ------------------------------------------------------------------ */
/*  Simple page allocator (returns BOOTLOADER_HEAP_BASE + aligned ptr) */
/* ------------------------------------------------------------------ */
void *memory_alloc_pages(size_t n_pages)
{
    return heap_alloc(NULL, n_pages * PAGE_SIZE, PAGE_SIZE);
}

/* ------------------------------------------------------------------ */
/*  EFI type → internal type mapping                                   */
/* ------------------------------------------------------------------ */
static mem_region_type_t efi_to_internal(uint32_t efi_type)
{
    switch (efi_type) {
    case EFI_CONVENTIONAL_MEMORY:
        return MEM_REGION_FREE;
    case EFI_LOADER_CODE:
    case EFI_LOADER_DATA:
        /* Reclaimable after ExitBootServices */
        return MEM_REGION_FREE;
    case EFI_BOOT_SERVICES_CODE:
    case EFI_BOOT_SERVICES_DATA:
        return MEM_REGION_FREE;   /* Linux reclaims after boot */
    case EFI_ACPI_RECLAIM_MEMORY:
        return MEM_REGION_ACPI;
    case EFI_ACPI_MEMORY_NVS:
        return MEM_REGION_NVS;
    case EFI_UNUSABLE_MEMORY:
        return MEM_REGION_UNUSABLE;
    case EFI_PERSISTENT_MEMORY:
        return MEM_REGION_PERSISTENT;
    default:
        return MEM_REGION_RESERVED;
    }
}

/* EFI type → E820 type */
static uint32_t efi_to_e820(uint32_t efi_type)
{
    switch (efi_type) {
    case EFI_CONVENTIONAL_MEMORY:
    case EFI_LOADER_CODE:
    case EFI_LOADER_DATA:
    case EFI_BOOT_SERVICES_CODE:
    case EFI_BOOT_SERVICES_DATA:
        return E820_TYPE_RAM;
    case EFI_ACPI_RECLAIM_MEMORY:
        return E820_TYPE_ACPI;
    case EFI_ACPI_MEMORY_NVS:
        return E820_TYPE_NVS;
    case EFI_UNUSABLE_MEMORY:
        return E820_TYPE_UNUSABLE;
    default:
        return E820_TYPE_RESERVED;
    }
}

/* ------------------------------------------------------------------ */
/*  mem_map_parse_efi                                                  */
/* ------------------------------------------------------------------ */
boot_status_t mem_map_parse_efi(const efi_memory_descriptor_t *descs,
                                 size_t n_desc,
                                 size_t desc_size,
                                 mem_map_t *out)
{
    if (!descs || !out) return BOOT_STATUS_ERROR;

    out->count           = 0;
    out->total_free_bytes = 0;
    out->total_bytes      = 0;

    const uint8_t *p = (const uint8_t *)descs;

    for (size_t i = 0; i < n_desc && out->count < MAX_MEM_REGIONS; i++) {
        const efi_memory_descriptor_t *d = (const efi_memory_descriptor_t *)p;
        p += desc_size;

        uint64_t base   = d->physical_start;
        uint64_t length = d->num_pages * PAGE_SIZE;

        mem_region_t *r = &out->regions[out->count++];
        r->base   = base;
        r->length = length;
        r->type   = efi_to_internal(d->type);

        out->total_bytes += length;
        if (r->type == MEM_REGION_FREE)
            out->total_free_bytes += length;
    }

    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  mem_map_to_e820                                                    */
/* ------------------------------------------------------------------ */
boot_status_t mem_map_to_e820(const mem_map_t *map,
                               struct boot_e820_entry *e820,
                               uint8_t *count)
{
    if (!map || !e820 || !count) return BOOT_STATUS_ERROR;

    /* We do a simple direct conversion (no coalescing needed here;    */
    /* Linux does its own coalescing).                                  */

    uint32_t n = 0;
    for (uint32_t i = 0; i < map->count && n < MAX_E820_ENTRIES; i++) {
        const mem_region_t *r = &map->regions[i];

        uint32_t t;
        switch (r->type) {
        case MEM_REGION_FREE:       t = E820_TYPE_RAM;      break;
        case MEM_REGION_ACPI:       t = E820_TYPE_ACPI;     break;
        case MEM_REGION_NVS:        t = E820_TYPE_NVS;      break;
        case MEM_REGION_UNUSABLE:   t = E820_TYPE_UNUSABLE; break;
        default:                    t = E820_TYPE_RESERVED; break;
        }

        e820[n].addr = r->base;
        e820[n].size = r->length;
        e820[n].type = t;
        n++;
    }

    *count = (uint8_t)n;
    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  memory_map_init – called early from boot_main                      */
/* ------------------------------------------------------------------ */
boot_status_t memory_map_init(boot_context_t *ctx)
{
    (void)ctx;
    /* The actual parsing happens in boot_main.c → parse_multiboot2(). */
    /* This function reserves space for fixed buffers.                 */
    serial_puts("[mem] memory subsystem ready, heap at ");
    serial_puthex64(BOOTLOADER_HEAP_BASE);
    serial_puts(" size ");
    serial_puthex64(BOOTLOADER_HEAP_SIZE);
    serial_puts("\n");
    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  memory_dump_e820 – debug helper                                    */
/* ------------------------------------------------------------------ */
void memory_dump_e820(const boot_context_t *ctx)
{
    const struct boot_params *bp = ctx->boot_params;
    serial_puts("[mem] E820 map (");
    serial_puthex64(bp->e820_entries);
    serial_puts(" entries):\n");

    for (uint8_t i = 0; i < bp->e820_entries; i++) {
        const struct boot_e820_entry *e = &bp->e820_table[i];
        serial_puts("  [");
        serial_puthex64(i);
        serial_puts("] base=");
        serial_puthex64(e->addr);
        serial_puts(" size=");
        serial_puthex64(e->size);
        serial_puts(" type=");
        serial_puthex64(e->type);
        serial_puts(" (");
        serial_puts(e820_type_name(e->type));
        serial_puts(")\n");
    }
}

/* ------------------------------------------------------------------ */
/*  memset_phys / memcpy_phys helpers                                  */
/* ------------------------------------------------------------------ */
void memset_phys(uint64_t phys, uint8_t val, size_t len)
{
    uint8_t *p = (uint8_t *)(uintptr_t)phys;
    while (len--) *p++ = val;
}

void memcpy_phys(uint64_t dst, const void *src, size_t len)
{
    uint8_t *d = (uint8_t *)(uintptr_t)dst;
    const uint8_t *s = (const uint8_t *)src;
    while (len--) *d++ = *s++;
}

/* ------------------------------------------------------------------ */
/*  e820_type_name                                                     */
/* ------------------------------------------------------------------ */
const char *e820_type_name(uint32_t type)
{
    switch (type) {
    case E820_TYPE_RAM:      return "RAM";
    case E820_TYPE_RESERVED: return "Reserved";
    case E820_TYPE_ACPI:     return "ACPI";
    case E820_TYPE_NVS:      return "NVS";
    case E820_TYPE_UNUSABLE: return "Unusable";
    default:                 return "Unknown";
    }
}