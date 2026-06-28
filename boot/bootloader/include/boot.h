/*
 * boot.h – MonoOS bootloader public types and declarations
 *
 * Shared by all C translation units in boot/bootloader/.
 * Also consumed by the Rust secure_boot and boot_manager crates via
 * cbindgen-generated wrappers.
 *
 * Linux x86-64 boot protocol reference:
 *   Documentation/x86/boot.rst (kernel source tree)
 */

#pragma once
#ifndef MONOOS_BOOT_H
#define MONOOS_BOOT_H

#include <stdint.h>
#include <stddef.h>

/* ------------------------------------------------------------------ */
/*  Compile-time version tag                                           */
/* ------------------------------------------------------------------ */
#define MONOOS_BOOTLOADER_VERSION_MAJOR  1
#define MONOOS_BOOTLOADER_VERSION_MINOR  0
#define MONOOS_BOOTLOADER_VERSION_PATCH  0

/* ------------------------------------------------------------------ */
/*  Physical memory layout constants                                   */
/*                                                                     */
/*  These must match the values in linker.ld and memory.h.            */
/*                                                                     */
/*  0x00000000 – 0x000FFFFF   Low 1 MiB (BIOS, real-mode, etc.)      */
/*  0x00010000                boot_params / zero page scratch          */
/*  0x00020000                Kernel command-line buffer               */
/*  0x00100000                Bootloader binary (loaded by GRUB)       */
/*  0x00200000                Linux protected-mode kernel image        */
/*  0x04000000                initramfs image                          */
/*  0x00110000                Bootloader bump-pointer heap             */
/* ------------------------------------------------------------------ */
#define BOOT_PARAMS_PHYS_ADDR    0x00010000ULL
#define CMDLINE_PHYS_ADDR        0x00020000ULL
#define CMDLINE_MAX              4096U
#define BOOTLOADER_LOAD_ADDR     0x00100000ULL
#define KERNEL_LOAD_ADDR         0x00200000UL
#define INITRAMFS_LOAD_ADDR      0x04000000ULL
#define BOOTLOADER_HEAP_BASE     0x00110000ULL
#define BOOTLOADER_HEAP_SIZE     (0x00100000ULL - 0x10000ULL) /* ~960 KiB */

/* Bootloader type identifier reported to kernel (0x72 = "Ox" type) */
#define BOOTLOADER_ID            0x72

/* ------------------------------------------------------------------ */
/*  Linux x86-64 boot protocol structures                              */
/*  Source: Linux include/uapi/linux/screen_info.h,                   */
/*          arch/x86/include/uapi/asm/bootparam.h                     */
/* ------------------------------------------------------------------ */

/* Setup header magic values */
#define SETUP_HEADER_OFFSET   0x1F1U
#define BOOT_FLAG_MAGIC       0xAA55U
#define LINUX_MAGIC           0x53726448U   /* "HdrS" little-endian */

/* Maximum E820 entries in boot_params */
#define MAX_E820_ENTRIES      128

/* E820 memory types */
#define E820_TYPE_RAM         1U
#define E820_TYPE_RESERVED    2U
#define E820_TYPE_ACPI        3U
#define E820_TYPE_NVS         4U
#define E820_TYPE_UNUSABLE    5U
#define E820_TYPE_PMEM        7U

/* screen_info (subset used) */
struct screen_info {
    uint8_t  orig_x;
    uint8_t  orig_y;
    uint16_t ext_mem_k;
    uint16_t orig_video_page;
    uint8_t  orig_video_mode;
    uint8_t  orig_video_cols;
    uint8_t  flags;
    uint8_t  unused2;
    uint16_t orig_video_ega_bx;
    uint16_t unused3;
    uint8_t  orig_video_lines;
    uint8_t  orig_video_isVGA;
    uint16_t orig_video_points;
    /* VESA / framebuffer fields */
    uint16_t lfb_width;
    uint16_t lfb_height;
    uint16_t lfb_depth;
    uint32_t lfb_base;
    uint32_t lfb_size;
    uint16_t cl_magic;
    uint16_t cl_offset;
    uint16_t lfb_linelength;
    uint8_t  red_size;
    uint8_t  red_pos;
    uint8_t  green_size;
    uint8_t  green_pos;
    uint8_t  blue_size;
    uint8_t  blue_pos;
    uint8_t  rsvd_size;
    uint8_t  rsvd_pos;
    uint16_t vesapm_seg;
    uint16_t vesapm_off;
    uint16_t pages;
    uint16_t vesa_attributes;
    uint32_t capabilities;
    uint32_t ext_lfb_base;
    uint8_t  _reserved[2];
} __attribute__((packed));

/* setup_header – matches Linux arch/x86/include/uapi/asm/bootparam.h */
struct setup_header {
    uint8_t   setup_sects;
    uint16_t  root_flags;
    uint32_t  syssize;
    uint16_t  ram_size;
    uint16_t  vid_mode;
    uint16_t  root_dev;
    uint16_t  boot_flag;       /* 0xAA55                               */
    uint16_t  jump;
    uint32_t  header;          /* "HdrS" = 0x53726448                  */
    uint16_t  version;         /* boot protocol version                */
    uint32_t  realmode_swtch;
    uint16_t  start_sys_seg;
    uint16_t  kernel_version;
    uint8_t   type_of_loader;
    uint8_t   loadflags;
    uint16_t  setup_move_size;
    uint32_t  code32_start;
    uint32_t  ramdisk_image;
    uint32_t  ramdisk_size;
    uint32_t  bootsect_kludge;
    uint16_t  heap_end_ptr;
    uint8_t   ext_loader_ver;
    uint8_t   ext_loader_type;
    uint32_t  cmd_line_ptr;
    uint32_t  initrd_addr_max;
    uint32_t  kernel_alignment;
    uint8_t   relocatable_kernel;
    uint8_t   min_alignment;
    uint16_t  xloadflags;
    uint32_t  cmdline_size;
    uint32_t  hardware_subarch;
    uint64_t  hardware_subarch_data;
    uint32_t  payload_offset;
    uint32_t  payload_length;
    uint64_t  setup_data;
    uint64_t  pref_address;
    uint32_t  init_size;
    uint32_t  handover_offset;
    uint32_t  kernel_info_offset;
} __attribute__((packed));

/* E820 entry */
struct boot_e820_entry {
    uint64_t addr;
    uint64_t size;
    uint32_t type;
} __attribute__((packed));

/* boot_params – the "zero page" passed to Linux */
struct boot_params {
    struct screen_info    screen_info;                    /* 0x000 */
    uint8_t               _pad0[0x040 - sizeof(struct screen_info)];
    uint8_t               apm_bios_info[0x14];           /* 0x040 */
    uint8_t               _pad1[0x060 - 0x040 - 0x14];
    uint8_t               hd0_info[16];                  /* 0x090 */
    uint8_t               hd1_info[16];                  /* 0x0A0 */
    uint8_t               _pad2[0x1C0 - 0x0B0];
    uint8_t               edid_info[0x80];               /* 0x140 */
    uint8_t               efi_info[0x20];                /* 0x1C0 */
    uint32_t              alt_mem_k;                     /* 0x1E0 */
    uint32_t              scratch;                       /* 0x1E4 */
    uint8_t               e820_entries;                  /* 0x1E8 */
    uint8_t               eddbuf_entries;                /* 0x1E9 */
    uint8_t               edd_mbr_sig_buf_entries;       /* 0x1EA */
    uint8_t               kbd_status;                    /* 0x1EB */
    uint8_t               secure_boot;                   /* 0x1EC */
    uint8_t               _pad3[2];
    uint8_t               sentinel;                      /* 0x1EF */
    uint8_t               _pad4[1];
    struct setup_header   hdr;                           /* 0x1F1 */
    uint8_t               _pad5[0x290 - 0x1F1 - sizeof(struct setup_header)];
    uint32_t              edd_mbr_sig_buffer[16];        /* 0x290 */
    struct boot_e820_entry e820_table[MAX_E820_ENTRIES]; /* 0x2D0 */
    uint8_t               _pad6[0xEEC - 0x2D0 - MAX_E820_ENTRIES * 20];
    uint32_t              ext_ramdisk_image;             /* 0xEEC */
    uint32_t              ext_ramdisk_size;              /* 0xEF0 */
    uint32_t              ext_cmd_line_ptr;              /* 0xEF4 */
} __attribute__((packed));

/* ------------------------------------------------------------------ */
/*  Boot status codes                                                   */
/* ------------------------------------------------------------------ */
typedef enum {
    BOOT_STATUS_OK          =  0,
    BOOT_STATUS_ERROR       = -1,
    BOOT_STATUS_NO_MEMORY   = -2,
    BOOT_STATUS_BAD_MAGIC   = -3,
    BOOT_STATUS_NOT_FOUND   = -4,
    BOOT_STATUS_VERIFY_FAIL = -5,
    BOOT_STATUS_IO_ERROR    = -6,
    BOOT_STATUS_TIMEOUT     = -7,
} boot_status_t;

/* ------------------------------------------------------------------ */
/*  Boot mode                                                           */
/* ------------------------------------------------------------------ */
typedef enum {
    BOOT_MODE_NORMAL      = 0,
    BOOT_MODE_RECOVERY    = 1,
    BOOT_MODE_FASTBOOT    = 2,
    BOOT_MODE_DIAGNOSTIC  = 3,
    BOOT_MODE_SAFE        = 4,   /* Minimal services, no 3rd-party drivers */
} boot_mode_t;

/* ------------------------------------------------------------------ */
/*  Kernel image descriptor                                            */
/* ------------------------------------------------------------------ */
typedef struct {
    void     *load_addr;     /* Virtual == physical in bootloader     */
    uint64_t  size;          /* Bytes                                 */
    uint64_t  entry_point;   /* Physical address of 64-bit entry      */
    uint32_t  setup_sects;   /* Number of setup sectors               */
    uint8_t   version_major;
    uint8_t   version_minor;
} kernel_image_t;

/* Initramfs descriptor */
typedef struct {
    void     *load_addr;
    uint64_t  size;
} initramfs_image_t;

/* ------------------------------------------------------------------ */
/*  Boot context – passed through the entire boot sequence            */
/* ------------------------------------------------------------------ */
typedef struct {
    struct boot_params  *boot_params;          /* Zero page            */
    boot_mode_t          mode;
    kernel_image_t       kernel;
    initramfs_image_t    initramfs;
    char                 cmdline[CMDLINE_MAX];
    int                  secure_boot_verified; /* 1 if signatures OK   */
    uint64_t             boot_time_us;         /* Microseconds at handoff */
} boot_context_t;

/* ------------------------------------------------------------------ */
/*  Function declarations (implemented across bootloader .c files)    */
/* ------------------------------------------------------------------ */

/* boot_main.c */
void          serial_init(void);
void          serial_puts(const char *s);
void          serial_puthex64(uint64_t v);
void __attribute__((noreturn)) boot_panic(const char *msg);
boot_status_t boot_main_init(boot_context_t *ctx);
void          boot_entry(uint64_t mb2_magic, uint64_t mb2_info_phys);

/* cpu_init.c */
void cpu_init_early(void);

/* memory_map.c */
boot_status_t memory_map_init(boot_context_t *ctx);
void          memory_dump_e820(const boot_context_t *ctx);
void          memset_phys(uint64_t phys, uint8_t val, size_t len);
void          memcpy_phys(uint64_t dst, const void *src, size_t len);
const char   *e820_type_name(uint32_t type);

/* kernel_loader.c */
boot_status_t kernel_loader_load(const char *path, boot_context_t *ctx);
boot_status_t __attribute__((noreturn)) kernel_loader_handoff(boot_context_t *ctx);

/* initramfs_loader.c */
boot_status_t initramfs_loader_load(const char *path, boot_context_t *ctx);

/* boot_args.c */
boot_status_t boot_args_build(boot_context_t *ctx, const char *extra_args);
const char   *boot_args_get(const boot_context_t *ctx);
const char   *boot_args_get_value(const boot_context_t *ctx, const char *key);
int           boot_args_has_flag(const boot_context_t *ctx, const char *flag);

#endif /* MONOOS_BOOT_H */
