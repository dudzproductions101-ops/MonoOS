/*
 * cpu_init.c – minimal CPU state setup
 *
 * GRUB 2 already enters 64-bit long mode before calling us, so most of
 * the work is done.  What we still need to ensure:
 *
 *   1. A clean, flat GDT is loaded (GRUB's GDT is fine but we install
 *      our own to make it self-contained).
 *   2. The IDT is cleared (we run with interrupts disabled; any NMI
 *      will triple-fault, which is acceptable for a bootloader).
 *   3. CR0/CR4 bits are correct for the Linux handoff:
 *        CR0: PE=1, MP=1, ET=1, NE=1, WP=1, AM=0, PG=1
 *        CR4: PAE=1, MCE=1, PGE=1 (if supported), OSFXSR+OSXMMEXCPT
 *   4. EFER.LME=1, EFER.NXE=1 (if supported).
 *
 * All these are already set by GRUB; we verify and re-set them to be
 * safe in case the firmware left something odd.
 */

#include "boot.h"
#include "memory.h"

#include <stdint.h>

/* ------------------------------------------------------------------ */
/*  GDT – 64-bit flat segments                                         */
/*                                                                     */
/*  We define three descriptors:                                       */
/*    0: null descriptor (required)                                    */
/*    1: 64-bit code segment (index 1, selector 0x08)                 */
/*    2: 64-bit data segment (index 2, selector 0x10)                 */
/* ------------------------------------------------------------------ */
#define GDT_NULL    0
#define GDT_CODE64  1
#define GDT_DATA64  2
#define GDT_ENTRIES 3

typedef struct {
    uint16_t limit_low;
    uint16_t base_low;
    uint8_t  base_mid;
    uint8_t  access;
    uint8_t  flags_limit_hi;
    uint8_t  base_hi;
} __attribute__((packed)) gdt_entry_t;

typedef struct {
    uint16_t  limit;
    uint64_t  base;
} __attribute__((packed)) gdtr_t;

/* GDT and GDTR aligned to 16 bytes */
static gdt_entry_t g_gdt[GDT_ENTRIES] __attribute__((aligned(16)));
static gdtr_t      g_gdtr;

static void gdt_set_entry(int idx, uint32_t base, uint32_t limit,
                           uint8_t access, uint8_t flags)
{
    g_gdt[idx].limit_low      = (uint16_t)(limit & 0xFFFF);
    g_gdt[idx].base_low       = (uint16_t)(base  & 0xFFFF);
    g_gdt[idx].base_mid       = (uint8_t)((base  >> 16) & 0xFF);
    g_gdt[idx].access         = access;
    g_gdt[idx].flags_limit_hi = (uint8_t)(flags << 4) |
                                  (uint8_t)((limit >> 16) & 0x0F);
    g_gdt[idx].base_hi        = (uint8_t)((base  >> 24) & 0xFF);
}

/* ------------------------------------------------------------------ */
/*  MSR helpers                                                         */
/* ------------------------------------------------------------------ */
#define MSR_EFER  0xC0000080

static uint64_t rdmsr(uint32_t msr)
{
    uint32_t lo, hi;
    __asm__ volatile ("rdmsr" : "=a"(lo), "=d"(hi) : "c"(msr));
    return ((uint64_t)hi << 32) | lo;
}

static void wrmsr(uint32_t msr, uint64_t val)
{
    uint32_t lo = (uint32_t)(val & 0xFFFFFFFF);
    uint32_t hi = (uint32_t)(val >> 32);
    __asm__ volatile ("wrmsr" : : "c"(msr), "a"(lo), "d"(hi));
}

/* ------------------------------------------------------------------ */
/*  CPUID helper                                                        */
/* ------------------------------------------------------------------ */
static void cpuid(uint32_t leaf,
                  uint32_t *eax, uint32_t *ebx,
                  uint32_t *ecx, uint32_t *edx)
{
    __asm__ volatile ("cpuid"
        : "=a"(*eax), "=b"(*ebx), "=c"(*ecx), "=d"(*edx)
        : "a"(leaf), "c"(0));
}

/* ------------------------------------------------------------------ */
/*  cpu_init_early                                                     */
/* ------------------------------------------------------------------ */
void cpu_init_early(void)
{
    /* ---- Install our own GDT ---- */
    /* Null descriptor */
    gdt_set_entry(GDT_NULL, 0, 0, 0, 0);

    /*
     * 64-bit code: access = Present | DPL0 | S | Execute | Read
     *              flags  = L (long mode) | G (granularity 4K)
     * access byte: 1001_1010b = 0x9A
     * flags nibble: 1010b = 0xA  (L=1, D=0, G=1)
     */
    gdt_set_entry(GDT_CODE64, 0, 0xFFFFF, 0x9A, 0xA);

    /*
     * 64-bit data: access = Present | DPL0 | S | Read/Write
     *              flags  = G (granularity 4K)
     * access byte: 1001_0010b = 0x92
     * flags nibble: 1000b = 0x8  (L=0, D=0, G=1) for data
     */
    gdt_set_entry(GDT_DATA64, 0, 0xFFFFF, 0x92, 0x8);

    g_gdtr.limit = sizeof(g_gdt) - 1;
    g_gdtr.base  = (uint64_t)(uintptr_t)g_gdt;

    __asm__ volatile ("lgdt %0" : : "m"(g_gdtr) : "memory");

    /* Reload segment registers.  In 64-bit mode DS/ES/SS/GS/FS are  */
    /* mostly ignored but must hold valid selectors.                   */
    __asm__ volatile (
        "movw $0x10, %%ax\n\t"
        "movw %%ax, %%ds\n\t"
        "movw %%ax, %%es\n\t"
        "movw %%ax, %%ss\n\t"
        "movw $0x00, %%ax\n\t"
        "movw %%ax, %%fs\n\t"
        "movw %%ax, %%gs\n\t"
        ::: "rax", "memory"
    );

    /* Far return to reload CS = 0x08 */
    __asm__ volatile (
        "pushq $0x08\n\t"
        "leaq 1f(%%rip), %%rax\n\t"
        "pushq %%rax\n\t"
        "lretq\n\t"
        "1:\n\t"
        ::: "rax", "memory"
    );

    /* ---- Clear IDT (we run with interrupts disabled) ---- */
    static gdtr_t null_idtr = { .limit = 0, .base = 0 };
    __asm__ volatile ("lidt %0" : : "m"(null_idtr) : "memory");

    /* ---- Ensure CR0 bits are correct ---- */
    uint64_t cr0;
    __asm__ volatile ("movq %%cr0, %0" : "=r"(cr0));
    /* PE=1, MP=1, ET=1, NE=1, WP=1, PG=1, clear AM (bit 18) */
    cr0 |=  (1ULL << 0)  | /* PE */
             (1ULL << 1)  | /* MP */
             (1ULL << 4)  | /* ET */
             (1ULL << 5)  | /* NE */
             (1ULL << 16) | /* WP */
             (1ULL << 31);  /* PG */
    cr0 &= ~(1ULL << 18);   /* Clear AM (alignment check) */
    __asm__ volatile ("movq %0, %%cr0" : : "r"(cr0) : "memory");

    /* ---- Ensure CR4 bits are correct ---- */
    uint64_t cr4;
    __asm__ volatile ("movq %%cr4, %0" : "=r"(cr4));

    /* PAE is required for 64-bit mode; should already be set */
    cr4 |= (1ULL << 5);  /* PAE */
    cr4 |= (1ULL << 6);  /* MCE – machine check enable */

    /* PGE (global pages) – check CPUID first */
    uint32_t a, b, c, d;
    cpuid(1, &a, &b, &c, &d);
    if (d & (1U << 13))  /* PGE supported */
        cr4 |= (1ULL << 7);

    /* OSFXSR and OSXMMEXCPT for SSE */
    if (d & (1U << 25))  /* SSE supported */
        cr4 |= (1ULL << 9) | (1ULL << 10);

    __asm__ volatile ("movq %0, %%cr4" : : "r"(cr4) : "memory");

    /* ---- EFER: ensure LME + LMA + NXE ---- */
    uint64_t efer = rdmsr(MSR_EFER);
    efer |= (1ULL << 8);   /* LME – Long Mode Enable   */
    efer |= (1ULL << 10);  /* LMA – Long Mode Active (RO, set by CPU) */

    /* NXE – check CPUID extended */
    cpuid(0x80000001, &a, &b, &c, &d);
    if (d & (1U << 20))    /* NX supported */
        efer |= (1ULL << 11);  /* NXE */

    wrmsr(MSR_EFER, efer);

    serial_puts("[cpu] long mode state verified\n");
}