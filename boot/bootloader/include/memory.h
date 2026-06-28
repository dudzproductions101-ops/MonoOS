/*
 * memory.h – MonoOS bootloader memory subsystem types and declarations
 *
 * Covers:
 *   - Page size and alignment macros
 *   - EFI memory descriptor types and structures
 *   - Internal memory-region enumeration
 *   - Bump-pointer heap API
 *   - mem_map_t (parsed from EFI/multiboot2 data)
 */

#pragma once
#ifndef MONOOS_MEMORY_H
#define MONOOS_MEMORY_H

#include <stdint.h>
#include <stddef.h>

/* ------------------------------------------------------------------ */
/*  Paging constants                                                   */
/* ------------------------------------------------------------------ */
#define PAGE_SHIFT          12U
#define PAGE_SIZE           (1ULL << PAGE_SHIFT)   /* 4096 bytes       */
#define PAGE_MASK           (~(PAGE_SIZE - 1ULL))
#define LARGE_PAGE_SIZE     (2ULL * 1024 * 1024)   /* 2 MiB            */
#define HUGE_PAGE_SIZE      (1ULL * 1024 * 1024 * 1024) /* 1 GiB       */

#define PAGE_ALIGN_UP(x)    (((uint64_t)(x) + PAGE_SIZE - 1) & PAGE_MASK)
#define PAGE_ALIGN_DOWN(x)  ((uint64_t)(x) & PAGE_MASK)
#define PAGES_FOR(bytes)    (((bytes) + PAGE_SIZE - 1) >> PAGE_SHIFT)

/* ------------------------------------------------------------------ */
/*  EFI memory type constants                                          */
/*  Source: UEFI Specification 2.10, Table 7-3                        */
/* ------------------------------------------------------------------ */
#define EFI_RESERVED_MEMORY_TYPE        0U
#define EFI_LOADER_CODE                 1U
#define EFI_LOADER_DATA                 2U
#define EFI_BOOT_SERVICES_CODE          3U
#define EFI_BOOT_SERVICES_DATA          4U
#define EFI_RUNTIME_SERVICES_CODE       5U
#define EFI_RUNTIME_SERVICES_DATA       6U
#define EFI_CONVENTIONAL_MEMORY         7U
#define EFI_UNUSABLE_MEMORY             8U
#define EFI_ACPI_RECLAIM_MEMORY         9U
#define EFI_ACPI_MEMORY_NVS             10U
#define EFI_MEMORY_MAPPED_IO            11U
#define EFI_MEMORY_MAPPED_IO_PORT_SPACE 12U
#define EFI_PAL_CODE                    13U
#define EFI_PERSISTENT_MEMORY           14U

/* EFI memory attribute flags */
#define EFI_MEMORY_UC   (1ULL << 0)   /* Uncacheable             */
#define EFI_MEMORY_WC   (1ULL << 1)   /* Write Combining         */
#define EFI_MEMORY_WT   (1ULL << 2)   /* Write Through           */
#define EFI_MEMORY_WB   (1ULL << 3)   /* Write Back (normal RAM) */
#define EFI_MEMORY_UCE  (1ULL << 4)   /* Uncacheable, exported   */
#define EFI_MEMORY_WP   (1ULL << 12)  /* Write Protected         */
#define EFI_MEMORY_RP   (1ULL << 13)  /* Read Protected          */
#define EFI_MEMORY_XP   (1ULL << 14)  /* Execute Protected       */
#define EFI_MEMORY_NV   (1ULL << 15)  /* Non-Volatile            */
#define EFI_MEMORY_MORE_RELIABLE (1ULL << 16)
#define EFI_MEMORY_RO   (1ULL << 17)  /* Read-Only               */
#define EFI_MEMORY_SP   (1ULL << 18)  /* Specific-Purpose        */
#define EFI_MEMORY_RUNTIME (1ULL << 63) /* Must be mapped at runtime */

/* EFI memory descriptor (48 bytes in UEFI 2.x) */
typedef struct {
    uint32_t type;
    uint32_t _pad;
    uint64_t physical_start;
    uint64_t virtual_start;
    uint64_t num_pages;
    uint64_t attribute;
} __attribute__((packed)) efi_memory_descriptor_t;

/* ------------------------------------------------------------------ */
/*  Internal memory region classification                              */
/* ------------------------------------------------------------------ */
typedef enum {
    MEM_REGION_FREE        = 0,  /* Available conventional RAM         */
    MEM_REGION_RESERVED    = 1,  /* Firmware/hardware reserved         */
    MEM_REGION_ACPI        = 2,  /* ACPI reclaimable                   */
    MEM_REGION_NVS         = 3,  /* ACPI Non-Volatile Storage          */
    MEM_REGION_UNUSABLE    = 4,  /* Memory with errors                 */
    MEM_REGION_PERSISTENT  = 5,  /* Persistent / NVDIMM                */
    MEM_REGION_BOOTLOADER  = 6,  /* Our own binary (reclaimable)       */
    MEM_REGION_KERNEL      = 7,  /* Loaded kernel image                */
    MEM_REGION_INITRAMFS   = 8,  /* Loaded initramfs                   */
    MEM_REGION_CMDLINE     = 9,  /* Kernel command-line buffer         */
    MEM_REGION_ZERO_PAGE   = 10, /* boot_params (zero page)            */
} mem_region_type_t;

/* Single memory region record */
typedef struct {
    uint64_t          base;
    uint64_t          length;
    mem_region_type_t type;
    uint64_t          efi_attributes; /* Original EFI attribute flags  */
} mem_region_t;

#define MAX_MEM_REGIONS 256

/* Parsed memory map (internal representation) */
typedef struct {
    mem_region_t regions[MAX_MEM_REGIONS];
    uint32_t     count;
    uint64_t     total_bytes;
    uint64_t     total_free_bytes;
} mem_map_t;

/* ------------------------------------------------------------------ */
/*  Bump-pointer heap (bootloader only – no free)                     */
/* ------------------------------------------------------------------ */
typedef struct {
    uint64_t base;
    uint64_t size;
    uint64_t used;
} heap_t;

/* ------------------------------------------------------------------ */
/*  Function declarations                                              */
/* ------------------------------------------------------------------ */

/* Heap */
void  heap_init(heap_t *h, uint64_t base, uint64_t size);
void *heap_alloc(heap_t *h, size_t bytes, size_t align);
void  heap_reset(heap_t *h);

/* Page-granular allocation from global heap */
void *memory_alloc_pages(size_t n_pages);

/* EFI → internal map parsing */
boot_status_t mem_map_parse_efi(const efi_memory_descriptor_t *descs,
                                 size_t n_desc,
                                 size_t desc_size,
                                 mem_map_t *out);

/* Internal map → E820 */
boot_status_t mem_map_to_e820(const mem_map_t *map,
                               struct boot_e820_entry *e820,
                               uint8_t *count);

#endif /* MONOOS_MEMORY_H */
