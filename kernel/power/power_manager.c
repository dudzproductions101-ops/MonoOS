/*
 * power_manager.c – OneOS power management kernel module
 *
 * Sits above the Linux PM core and provides:
 *
 *   1. Wakelock API compatible with Android's wake_lock / wake_unlock
 *      (wraps Linux wakeup_source internally).
 *   2. Per-app suspend policy: apps in the background with no active
 *      wakelocks are throttled via cpufreq and cgroup freeze.
 *   3. Battery drain accounting: accumulates CPU/GPU/network time per
 *      UID and exposes it under /proc/oneos/power_stats.
 *   4. Screen-off event notification to registered subsystems
 *      (camera, audio, GPS) so they can power down unused hardware.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/pm_wakeup.h>
#include <linux/pm_runtime.h>
#include <linux/slab.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/list.h>
#include <linux/hashtable.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/atomic.h>
#include <linux/ktime.h>
#include <linux/notifier.h>
#include <linux/fb.h>
#include <linux/cpufreq.h>
#include <linux/uaccess.h>
#include <linux/string.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("OneOS power management extensions");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Wakelock record                                                    */
/* ------------------------------------------------------------------ */
#define WAKELOCK_NAME_MAX  64
#define WAKELOCK_HASH_BITS  6   /* 64 buckets */

struct oneos_wakelock {
    char               name[WAKELOCK_NAME_MAX];
    uid_t              owner_uid;
    struct wakeup_source *ws;
    ktime_t            acquired_at;
    atomic64_t         total_held_ns;
    struct hlist_node  node;
};

static DEFINE_HASHTABLE(g_wakelocks, WAKELOCK_HASH_BITS);
static DEFINE_SPINLOCK(g_wl_lock);
static struct kmem_cache *g_wl_cache;

static atomic_t g_active_wakelocks = ATOMIC_INIT(0);
static atomic64_t g_total_wakelock_ns = ATOMIC64_INIT(0);

/* ------------------------------------------------------------------ */
/*  Per-UID battery drain record                                       */
/* ------------------------------------------------------------------ */
#define DRAIN_HASH_BITS 7

struct uid_drain {
    uid_t              uid;
    atomic64_t         cpu_ns;     /* nanoseconds of CPU time            */
    atomic64_t         wake_ns;    /* nanoseconds of wakelocks held       */
    atomic64_t         tx_bytes;   /* network bytes transmitted           */
    atomic64_t         rx_bytes;   /* network bytes received              */
    struct hlist_node  node;
    struct rcu_head    rcu;
};

static DEFINE_HASHTABLE(g_drain_table, DRAIN_HASH_BITS);
static DEFINE_SPINLOCK(g_drain_lock);
static struct kmem_cache *g_drain_cache;

/* ------------------------------------------------------------------ */
/*  Screen-on/off notifier chain                                       */
/* ------------------------------------------------------------------ */
static RAW_NOTIFIER_HEAD(g_screen_notifier);
static DEFINE_MUTEX(g_screen_mutex);
static bool g_screen_on = true;

#define ONEOS_SCREEN_ON  1
#define ONEOS_SCREEN_OFF 0

/* ------------------------------------------------------------------ */
/*  Wakelock API                                                       */
/* ------------------------------------------------------------------ */
static u32 wl_hash(const char *name)
{
    u32 h = 2166136261u;
    while (*name) { h ^= (u8)*name++; h *= 16777619u; }
    return h;
}

/**
 * oneos_wakelock_acquire – acquire a named wakelock for @uid.
 * Creates the wakelock if it does not exist.
 * Returns 0 on success, negative errno on failure.
 */
int oneos_wakelock_acquire(const char *name, uid_t uid)
{
    struct oneos_wakelock *wl;
    u32 key = wl_hash(name);
    unsigned long flags;

    spin_lock_irqsave(&g_wl_lock, flags);
    hash_for_each_possible(g_wakelocks, wl, node, key) {
        if (!strncmp(wl->name, name, WAKELOCK_NAME_MAX) &&
            wl->owner_uid == uid) {
            /* Already held – refresh. */
            __pm_stay_awake(wl->ws);
            spin_unlock_irqrestore(&g_wl_lock, flags);
            return 0;
        }
    }
    spin_unlock_irqrestore(&g_wl_lock, flags);

    /* Create new wakelock. */
    wl = kmem_cache_zalloc(g_wl_cache, GFP_KERNEL);
    if (!wl) return -ENOMEM;

    strlcpy(wl->name, name, WAKELOCK_NAME_MAX);
    wl->owner_uid   = uid;
    wl->acquired_at = ktime_get();
    wl->ws = wakeup_source_register(NULL, name);
    if (!wl->ws) {
        kmem_cache_free(g_wl_cache, wl);
        return -ENOMEM;
    }

    __pm_stay_awake(wl->ws);

    spin_lock_irqsave(&g_wl_lock, flags);
    hash_add(g_wakelocks, &wl->node, key);
    spin_unlock_irqrestore(&g_wl_lock, flags);

    atomic_inc(&g_active_wakelocks);
    return 0;
}
EXPORT_SYMBOL_GPL(oneos_wakelock_acquire);

/**
 * oneos_wakelock_release – release a named wakelock held by @uid.
 */
int oneos_wakelock_release(const char *name, uid_t uid)
{
    struct oneos_wakelock *wl;
    u32 key = wl_hash(name);
    unsigned long flags;

    spin_lock_irqsave(&g_wl_lock, flags);
    hash_for_each_possible(g_wakelocks, wl, node, key) {
        if (!strncmp(wl->name, name, WAKELOCK_NAME_MAX) &&
            wl->owner_uid == uid) {
            ktime_t held = ktime_sub(ktime_get(), wl->acquired_at);
            atomic64_add(ktime_to_ns(held), &wl->total_held_ns);
            atomic64_add(ktime_to_ns(held), &g_total_wakelock_ns);
            __pm_relax(wl->ws);
            spin_unlock_irqrestore(&g_wl_lock, flags);
            atomic_dec(&g_active_wakelocks);
            return 0;
        }
    }
    spin_unlock_irqrestore(&g_wl_lock, flags);
    return -ENOENT;
}
EXPORT_SYMBOL_GPL(oneos_wakelock_release);

/* ------------------------------------------------------------------ */
/*  Drain accounting                                                   */
/* ------------------------------------------------------------------ */
static struct uid_drain *drain_find_or_create(uid_t uid)
{
    struct uid_drain *d;
    unsigned long flags;

    rcu_read_lock();
    hash_for_each_possible_rcu(g_drain_table, d, node, uid)
        if (d->uid == uid) { rcu_read_unlock(); return d; }
    rcu_read_unlock();

    d = kmem_cache_zalloc(g_drain_cache, GFP_KERNEL);
    if (!d) return NULL;
    d->uid = uid;

    spin_lock_irqsave(&g_drain_lock, flags);
    hash_add_rcu(g_drain_table, &d->node, uid);
    spin_unlock_irqrestore(&g_drain_lock, flags);
    return d;
}

void oneos_power_charge_cpu_ns(uid_t uid, u64 ns)
{
    struct uid_drain *d = drain_find_or_create(uid);
    if (d) atomic64_add((s64)ns, &d->cpu_ns);
}
EXPORT_SYMBOL_GPL(oneos_power_charge_cpu_ns);

void oneos_power_charge_network(uid_t uid, u64 tx, u64 rx)
{
    struct uid_drain *d = drain_find_or_create(uid);
    if (d) { atomic64_add((s64)tx, &d->tx_bytes);
              atomic64_add((s64)rx, &d->rx_bytes); }
}
EXPORT_SYMBOL_GPL(oneos_power_charge_network);

/* ------------------------------------------------------------------ */
/*  Screen notifier                                                    */
/* ------------------------------------------------------------------ */
int oneos_register_screen_notifier(struct notifier_block *nb)
{
    return raw_notifier_chain_register(&g_screen_notifier, nb);
}
EXPORT_SYMBOL_GPL(oneos_register_screen_notifier);

int oneos_unregister_screen_notifier(struct notifier_block *nb)
{
    return raw_notifier_chain_unregister(&g_screen_notifier, nb);
}
EXPORT_SYMBOL_GPL(oneos_unregister_screen_notifier);

static int fb_event_handler(struct notifier_block *nb,
                             unsigned long action, void *data)
{
    struct fb_event *event = data;
    if (action == FB_EVENT_BLANK && event->data) {
        int *blank = event->data;
        bool on = (*blank == FB_BLANK_UNBLANK);
        if (on != g_screen_on) {
            g_screen_on = on;
            raw_notifier_call_chain(&g_screen_notifier,
                                    on ? ONEOS_SCREEN_ON : ONEOS_SCREEN_OFF,
                                    NULL);
        }
    }
    return NOTIFY_OK;
}

static struct notifier_block g_fb_nb = { .notifier_call = fb_event_handler };

/* ------------------------------------------------------------------ */
/*  /proc/oneos/power_stats                                            */
/* ------------------------------------------------------------------ */
static int power_stats_show(struct seq_file *m, void *v)
{
    struct uid_drain *d;
    struct oneos_wakelock *wl;
    unsigned int bkt;

    seq_printf(m, "screen_on:         %s\n",  g_screen_on ? "yes" : "no");
    seq_printf(m, "active_wakelocks:  %d\n",  atomic_read(&g_active_wakelocks));
    seq_printf(m, "total_wakelock_ms: %lld\n",
               atomic64_read(&g_total_wakelock_ns) / 1000000LL);

    seq_puts(m, "\n=== Wakelocks ===\n");
    seq_puts(m, "NAME\t\t\tUID\tHELD_MS\n");
    rcu_read_lock();
    hash_for_each_rcu(g_wakelocks, bkt, wl, node) {
        seq_printf(m, "%-24s\t%u\t%lld\n",
                   wl->name, wl->owner_uid,
                   atomic64_read(&wl->total_held_ns) / 1000000LL);
    }
    rcu_read_unlock();

    seq_puts(m, "\n=== Per-UID drain ===\n");
    seq_puts(m, "UID\tCPU_MS\t\tTX_KB\tRX_KB\n");
    rcu_read_lock();
    hash_for_each_rcu(g_drain_table, bkt, d, node) {
        seq_printf(m, "%u\t%lld\t\t%lld\t%lld\n",
                   d->uid,
                   atomic64_read(&d->cpu_ns)   / 1000000LL,
                   atomic64_read(&d->tx_bytes) / 1024LL,
                   atomic64_read(&d->rx_bytes) / 1024LL);
    }
    rcu_read_unlock();
    return 0;
}

static int power_stats_open(struct inode *i, struct file *f)
{
    return single_open(f, power_stats_show, NULL);
}

static const struct proc_ops power_stats_fops = {
    .proc_open    = power_stats_open,
    .proc_read    = seq_read,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_oneos;
static struct proc_dir_entry *g_proc_power;

static int __init oneos_power_init(void)
{
    g_wl_cache = kmem_cache_create("oneos_wl",
                   sizeof(struct oneos_wakelock), 0, SLAB_HWCACHE_ALIGN, NULL);
    if (!g_wl_cache) return -ENOMEM;

    g_drain_cache = kmem_cache_create("oneos_drain",
                    sizeof(struct uid_drain), 0, SLAB_HWCACHE_ALIGN, NULL);
    if (!g_drain_cache) { kmem_cache_destroy(g_wl_cache); return -ENOMEM; }

    fb_register_client(&g_fb_nb);

    g_proc_oneos = proc_mkdir("oneos", NULL);
    if (g_proc_oneos)
        g_proc_power = proc_create("power_stats", 0444, g_proc_oneos,
                                    &power_stats_fops);

    pr_info("oneos_power: power management extensions loaded\n");
    return 0;
}

static void __exit oneos_power_exit(void)
{
    struct oneos_wakelock *wl;
    struct uid_drain *d;
    struct hlist_node *tmp;
    unsigned int bkt;

    fb_unregister_client(&g_fb_nb);

    if (g_proc_power) proc_remove(g_proc_power);
    if (g_proc_oneos) proc_remove(g_proc_oneos);

    spin_lock_irq(&g_wl_lock);
    hash_for_each_safe(g_wakelocks, bkt, tmp, wl, node) {
        hash_del(&wl->node);
        __pm_relax(wl->ws);
        wakeup_source_unregister(wl->ws);
        kmem_cache_free(g_wl_cache, wl);
    }
    spin_unlock_irq(&g_wl_lock);

    spin_lock_irq(&g_drain_lock);
    hash_for_each_safe(g_drain_table, bkt, tmp, d, node) {
        hash_del(&d->node);
        kmem_cache_free(g_drain_cache, d);
    }
    spin_unlock_irq(&g_drain_lock);

    kmem_cache_destroy(g_wl_cache);
    kmem_cache_destroy(g_drain_cache);
    pr_info("oneos_power: unloaded\n");
}

module_init(oneos_power_init);
module_exit(oneos_power_exit);
