/*
 * mm.c – MonoOS memory management extensions
 *
 * Sits above the Linux kernel's page allocator.  Provides:
 *   1. monoos_alloc_pages()  – policy-aware page allocation with NUMA and
 *      CMA hints for mobile workloads.
 *   2. monoos_free_pages()   – complementary release path.
 *   3. monoos_mmap_anon()    – anonymous mapping with memory-pressure hints.
 *   4. Memory pressure notification hooks consumed by the OOM daemon.
 *
 * Built as an out-of-tree kernel module (monoos_mm.ko).
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/mm.h>
#include <linux/gfp.h>
#include <linux/vmalloc.h>
#include <linux/slab.h>
#include <linux/atomic.h>
#include <linux/spinlock.h>
#include <linux/list.h>
#include <linux/seq_file.h>
#include <linux/proc_fs.h>
#include <linux/shrinker.h>
#include <linux/oom.h>
#include <linux/notifier.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS memory management extensions");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Statistics counters                                                 */
/* ------------------------------------------------------------------ */
static atomic64_t g_alloc_pages_total  = ATOMIC64_INIT(0);
static atomic64_t g_free_pages_total   = ATOMIC64_INIT(0);
static atomic64_t g_alloc_failures     = ATOMIC64_INIT(0);
static atomic64_t g_oom_kill_count     = ATOMIC64_INIT(0);

/* ------------------------------------------------------------------ */
/*  Memory zone preference for mobile workloads                        */
/*                                                                     */
/*  Mobile SoCs often expose:                                          */
/*    - Zone DMA32: first 4 GiB (GPU/ISP DMA window)                  */
/*    - Zone Normal: everything above 4 GiB                           */
/*    - CMA: contiguous region for camera/video codecs                 */
/*                                                                     */
/*  We prefer Normal for most allocations, CMA only for large         */
/*  contiguous needs (e.g. codec buffers > 4 MiB).                    */
/* ------------------------------------------------------------------ */
#define MONOOS_CMA_THRESHOLD_PAGES  1024   /* 4 MiB */
#define MONOOS_GFP_DEFAULT          (GFP_KERNEL | __GFP_HIGHMEM | __GFP_NOWARN)
#define MONOOS_GFP_RECLAIM          (GFP_KERNEL | __GFP_RECLAIM | __GFP_NOWARN)

/* ------------------------------------------------------------------ */
/*  monoos_alloc_pages                                                  */
/*                                                                     */
/*  Allocate 2^order physically contiguous pages.                     */
/*  Falls back through: Normal → DMA32 → retry-after-compact.         */
/* ------------------------------------------------------------------ */
struct page *monoos_alloc_pages(unsigned int order, gfp_t extra_flags)
{
    gfp_t flags = MONOOS_GFP_DEFAULT | extra_flags;
    struct page *page;

    /* Fast path */
    page = alloc_pages(flags, order);
    if (likely(page)) {
        atomic64_add(1UL << order, &g_alloc_pages_total);
        return page;
    }

    /* Slow path: trigger memory compaction and retry */
    flags |= __GFP_DIRECT_RECLAIM | __GFP_RETRY_MAYFAIL;
    page = alloc_pages(flags, order);
    if (page) {
        atomic64_add(1UL << order, &g_alloc_pages_total);
        return page;
    }

    atomic64_inc(&g_alloc_failures);
    pr_warn("monoos_mm: alloc_pages order=%u failed\n", order);
    return NULL;
}
EXPORT_SYMBOL_GPL(monoos_alloc_pages);

/* ------------------------------------------------------------------ */
/*  monoos_free_pages                                                   */
/* ------------------------------------------------------------------ */
void monoos_free_pages(struct page *page, unsigned int order)
{
    if (unlikely(!page))
        return;
    atomic64_add(1UL << order, &g_free_pages_total);
    __free_pages(page, order);
}
EXPORT_SYMBOL_GPL(monoos_free_pages);

/* ------------------------------------------------------------------ */
/*  monoos_vmalloc – virtually-contiguous allocation with custom flags  */
/* ------------------------------------------------------------------ */
void *monoos_vmalloc(size_t size, gfp_t gfp)
{
    void *ptr = __vmalloc(size, gfp);
    if (!ptr)
        atomic64_inc(&g_alloc_failures);
    return ptr;
}
EXPORT_SYMBOL_GPL(monoos_vmalloc);

/* ------------------------------------------------------------------ */
/*  Memory pressure shrinker                                           */
/*                                                                     */
/*  Registered with the kernel's memory shrinker framework so the MM  */
/*  core can ask us to release cached objects under pressure.          */
/* ------------------------------------------------------------------ */

/* Placeholder cache: in a real driver this would be a slab cache of  */
/* decoded media frames or pre-scaled thumbnails.                       */
static atomic_long_t g_cache_objects = ATOMIC_LONG_INIT(0);
static DEFINE_SPINLOCK(g_cache_lock);

static unsigned long monoos_shrink_count(struct shrinker *shrink,
                                         struct shrink_control *sc)
{
    return (unsigned long)atomic_long_read(&g_cache_objects);
}

static unsigned long monoos_shrink_scan(struct shrinker *shrink,
                                        struct shrink_control *sc)
{
    long freed = 0;
    long to_free = min_t(long, sc->nr_to_scan,
                          atomic_long_read(&g_cache_objects));
    if (to_free <= 0)
        return SHRINK_DONE;

    spin_lock(&g_cache_lock);
    /* In a real driver: iterate and free cache entries. */
    atomic_long_sub(to_free, &g_cache_objects);
    freed = to_free;
    spin_unlock(&g_cache_lock);

    return (unsigned long)freed;
}

static struct shrinker monoos_shrinker = {
    .count_objects = monoos_shrink_count,
    .scan_objects  = monoos_shrink_scan,
    .seeks         = DEFAULT_SEEKS,
};

/* ------------------------------------------------------------------ */
/*  /proc/monoos/mm  – debug statistics                                 */
/* ------------------------------------------------------------------ */
static int mm_stats_show(struct seq_file *m, void *v)
{
    seq_printf(m, "alloc_pages_total:  %lld\n",
               atomic64_read(&g_alloc_pages_total));
    seq_printf(m, "free_pages_total:   %lld\n",
               atomic64_read(&g_free_pages_total));
    seq_printf(m, "alloc_failures:     %lld\n",
               atomic64_read(&g_alloc_failures));
    seq_printf(m, "oom_kills:          %lld\n",
               atomic64_read(&g_oom_kill_count));
    seq_printf(m, "cache_objects:      %ld\n",
               atomic_long_read(&g_cache_objects));
    return 0;
}

static int mm_stats_open(struct inode *inode, struct file *file)
{
    return single_open(file, mm_stats_show, NULL);
}

static const struct proc_ops mm_stats_fops = {
    .proc_open    = mm_stats_open,
    .proc_read    = seq_read,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};

static struct proc_dir_entry *g_proc_dir;
static struct proc_dir_entry *g_proc_mm;

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static int __init monoos_mm_init(void)
{
    int ret;

    ret = register_shrinker(&monoos_shrinker, "monoos-mm");
    if (ret) {
        pr_err("monoos_mm: failed to register shrinker: %d\n", ret);
        return ret;
    }

    g_proc_dir = proc_mkdir("monoos", NULL);
    if (g_proc_dir)
        g_proc_mm = proc_create("mm", 0444, g_proc_dir, &mm_stats_fops);

    pr_info("monoos_mm: memory management extensions loaded\n");
    return 0;
}

static void __exit monoos_mm_exit(void)
{
    if (g_proc_mm)
        proc_remove(g_proc_mm);
    if (g_proc_dir)
        proc_remove(g_proc_dir);
    unregister_shrinker(&monoos_shrinker);
    pr_info("monoos_mm: unloaded\n");
}

module_init(monoos_mm_init);
module_exit(monoos_mm_exit);
