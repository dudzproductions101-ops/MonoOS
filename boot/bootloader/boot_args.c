/*
 * boot_args.c – construct and manage the Linux kernel command line
 *
 * The kernel command line is a NUL-terminated ASCII string placed at a
 * fixed physical address (CMDLINE_PHYS_ADDR).  The boot protocol
 * requires:
 *
 *   boot_params->hdr.cmd_line_ptr  = low  32 bits of cmdline address
 *   boot_params->ext_cmd_line_ptr  = high 32 bits (almost always 0)
 *
 * Maximum size of the command line is given by
 *   boot_params->hdr.cmdline_size (set by kernel; 0 means 255 bytes)
 *
 * We also provide a simple key=value parser so later code can query
 * individual parameters (e.g. "boot_mode=recovery").
 */

#include "boot.h"
#include "memory.h"

#include <stdint.h>
#include <stddef.h>

/* ------------------------------------------------------------------ */
/*  String helpers                                                      */
/* ------------------------------------------------------------------ */
static size_t slen(const char *s)
{
    if (!s) return 0;
    size_t n = 0;
    while (s[n]) n++;
    return n;
}

static void scat(char *dst, size_t dst_cap, const char *src)
{
    size_t dl = slen(dst);
    size_t sl = slen(src);
    if (dl + sl + 1 > dst_cap) {
        sl = dst_cap - dl - 1;
    }
    for (size_t i = 0; i < sl; i++) dst[dl + i] = src[i];
    dst[dl + sl] = '\0';
}

static void scpy(char *dst, size_t cap, const char *src)
{
    size_t i = 0;
    while (src[i] && i < cap - 1) {
        dst[i] = src[i];
        i++;
    }
    dst[i] = '\0';
}

static int smatch(const char *a, const char *b, size_t n)
{
    for (size_t i = 0; i < n; i++) {
        if (a[i] != b[i]) return 0;
        if (a[i] == '\0') return 1;
    }
    return 1;
}

/* ------------------------------------------------------------------ */
/*  MonoOS default base command line                                    */
/*                                                                     */
/*  This is merged with the caller-supplied extra_args and any        */
/*  parameters derived from the boot mode/flags.                       */
/* ------------------------------------------------------------------ */
#define BASE_CMDLINE \
    "ro " \
    "console=ttyS0,115200n8 " \
    "console=tty0 " \
    "loglevel=4 " \
    "panic=5 " \
    "monoos.version=1.0"

/* ------------------------------------------------------------------ */
/*  boot_args_build                                                    */
/* ------------------------------------------------------------------ */
boot_status_t boot_args_build(boot_context_t *ctx, const char *extra_args)
{
    struct boot_params *bp = ctx->boot_params;

    /* Physical address where we store the command line */
    char *cmdline_phys = (char *)(uintptr_t)CMDLINE_PHYS_ADDR;

    /* Start with the base */
    scpy(cmdline_phys, CMDLINE_MAX, BASE_CMDLINE);

    /* Append a space separator before extra args */
    if (extra_args && extra_args[0] != '\0') {
        scat(cmdline_phys, CMDLINE_MAX, " ");
        scat(cmdline_phys, CMDLINE_MAX, extra_args);
    }

    /* Append boot-mode-specific parameters */
    switch (ctx->mode) {
    case BOOT_MODE_RECOVERY:
        scat(cmdline_phys, CMDLINE_MAX,
             " monoos.mode=recovery systemd.unit=recovery.target");
        break;
    case BOOT_MODE_DIAGNOSTIC:
        scat(cmdline_phys, CMDLINE_MAX,
             " monoos.mode=diagnostic loglevel=7 systemd.unit=diagnostic.target");
        break;
    case BOOT_MODE_FASTBOOT:
        scat(cmdline_phys, CMDLINE_MAX,
             " monoos.mode=fastboot");
        break;
    case BOOT_MODE_NORMAL:
    default:
        scat(cmdline_phys, CMDLINE_MAX,
             " monoos.mode=normal");
        break;
    }

    /* Store a copy in the context struct for Rust modules to read     */
    scpy(ctx->cmdline, CMDLINE_MAX, cmdline_phys);

    /* Populate boot_params */
    uint64_t phys64 = CMDLINE_PHYS_ADDR;
    bp->hdr.cmd_line_ptr  = (uint32_t)(phys64 & 0xFFFFFFFF);
    bp->ext_cmd_line_ptr  = (uint32_t)(phys64 >> 32);

    /* If the kernel told us its max cmdline size, respect it */
    uint32_t kmax = bp->hdr.cmdline_size;
    if (kmax == 0) kmax = 255;

    size_t actual = slen(cmdline_phys);
    if (actual >= kmax) {
        serial_puts("[args] Warning: cmdline truncated by kernel limit\n");
    }

    serial_puts("[args] cmdline: ");
    serial_puts(cmdline_phys);
    serial_puts("\n");

    return BOOT_STATUS_OK;
}

/* ------------------------------------------------------------------ */
/*  boot_args_get – return pointer to the built command line           */
/* ------------------------------------------------------------------ */
const char *boot_args_get(const boot_context_t *ctx)
{
    return ctx->cmdline;
}

/* ------------------------------------------------------------------ */
/*  boot_args_get_value – find the value of "key=..." in cmdline       */
/*                                                                     */
/*  Returns pointer into the cmdline string, or NULL if not found.    */
/*  The returned pointer is to the character immediately after '='.   */
/* ------------------------------------------------------------------ */
const char *boot_args_get_value(const boot_context_t *ctx, const char *key)
{
    const char *p = ctx->cmdline;
    size_t klen   = slen(key);

    while (*p) {
        /* Skip whitespace */
        while (*p == ' ' || *p == '\t') p++;

        /* Check if this token starts with key= */
        if (smatch(p, key, klen) && p[klen] == '=') {
            return p + klen + 1;
        }

        /* Skip to next whitespace */
        while (*p && *p != ' ' && *p != '\t') p++;
    }

    return NULL;
}

/* ------------------------------------------------------------------ */
/*  boot_args_has_flag – check for a boolean flag in cmdline           */
/* ------------------------------------------------------------------ */
int boot_args_has_flag(const boot_context_t *ctx, const char *flag)
{
    const char *p = ctx->cmdline;
    size_t flen   = slen(flag);

    while (*p) {
        while (*p == ' ' || *p == '\t') p++;

        if (smatch(p, flag, flen)) {
            char next = p[flen];
            if (next == '\0' || next == ' ' || next == '\t') {
                return 1;
            }
        }

        while (*p && *p != ' ' && *p != '\t') p++;
    }

    return 0;
}