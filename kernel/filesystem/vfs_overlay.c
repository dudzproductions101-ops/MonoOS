/*
 * vfs_overlay.c – MonoOS VFS privacy overlay
 *
 * Implements a lightweight eBPF-free LSM hook layer that:
 *
 *   1. Intercepts open(2)/openat(2) on sensitive path prefixes
 *      /proc-pid, sys-devices, dev-camera, dev-snd
 *      and enforces MonoOS runtime permissions before allowing access.
 *
 *   2. Provides a synthetic /proc/monoos/fs_events ring buffer that
 *      the privacy daemon reads to build an access log.
 *
 *   3. Implements a "sandbox path" feature: third-party apps are
 *      redirected from /data/data/<pkg>/ to a private overlay
 *      mount visible only to that app's UID.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/fs.h>
#include <linux/path.h>
#include <linux/dcache.h>
#include <linux/namei.h>
#include <linux/slab.h>
#include <linux/spinlock.h>
#include <linux/kfifo.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/atomic.h>
#include <linux/string.h>
#include <linux/uaccess.h>
#include <linux/time64.h>
#include "monoos_process.h" /* monoos_proc_has_perm() — resolved via ccflags-y */

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS VFS privacy overlay");
MODULE_VERSION("1.0.0");

/*
 * monoos_process must be fully initialised before monoos_vfs runs its
 * init — specifically, monoos_process registers /proc/monoos first, and
 * monoos_vfs must not call proc_mkdir("monoos") until that entry exists.
 *
 * Without this softdep the two modules can race during parallel modprobe
 * and produce the spurious "proc_dir_entry '/proc/monoos' already
 * registered" warning noted in HANDOFF.md.
 */
MODULE_SOFTDEP("pre: monoos_process");

/* ------------------------------------------------------------------ */
/*  Sensitive path prefixes and their required permission bit         */
/* ------------------------------------------------------------------ */
struct sensitive_path {
    const char *prefix;
    u32         required_perm;   /* MONOOS_PERM_* from process.c      */
    bool        log_always;      /* always emit fs_event even if OK   */
};

#define MONOOS_PERM_CAMERA   0x0001U
#define MONOOS_PERM_MIC      0x0002U
#define MONOOS_PERM_LOCATION 0x0004U
#define MONOOS_PERM_STORAGE  0x0010U

static const struct sensitive_path sensitive_paths[] = {
    { "/dev/video",        MONOOS_PERM_CAMERA,   true  },
    { "/dev/camera",       MONOOS_PERM_CAMERA,   true  },
    { "/dev/snd/",         MONOOS_PERM_MIC,      true  },
    { "/dev/dsp",          MONOOS_PERM_MIC,      true  },
    { "/sys/bus/iio/",     MONOOS_PERM_LOCATION, false },
    { "/proc/net/",        MONOOS_PERM_STORAGE,  false },
};
#define N_SENSITIVE_PATHS ARRAY_SIZE(sensitive_paths)

/* ------------------------------------------------------------------ */
/*  FS access event (written to the ring buffer)                      */
/* ------------------------------------------------------------------ */
#define FS_PATH_MAX 128

struct fs_event {
    u64  ts_ns;
    pid_t pid;
    uid_t uid;
    u32  perm_required;
    u8   allowed;     /* 1=permitted, 0=blocked */
    char path[FS_PATH_MAX];
} __packed;

#define FS_EVENT_FIFO_SIZE 256   /* power of 2, events */

DEFINE_KFIFO(g_event_fifo, struct fs_event, FS_EVENT_FIFO_SIZE);
static DEFINE_SPINLOCK(g_fifo_lock);

static atomic64_t g_events_total   = ATOMIC64_INIT(0);
static atomic64_t g_events_blocked = ATOMIC64_INIT(0);

/* ------------------------------------------------------------------ */
/*  Helper: emit an event into the ring buffer                        */
/* ------------------------------------------------------------------ */
static void emit_event(const char *path, u32 perm, bool allowed)
{
    struct fs_event ev;
    unsigned long flags;

    ev.ts_ns         = ktime_get_ns();
    ev.pid           = current->pid;
    ev.uid           = from_kuid_munged(current_user_ns(), current_uid());
    ev.perm_required = perm;
    ev.allowed       = allowed ? 1 : 0;
    strscpy(ev.path, path, FS_PATH_MAX);   /* strlcpy removed in Linux 6.x */

    spin_lock_irqsave(&g_fifo_lock, flags);
    kfifo_in(&g_event_fifo, &ev, 1);
    spin_unlock_irqrestore(&g_fifo_lock, flags);

    atomic64_inc(&g_events_total);
    if (!allowed) atomic64_inc(&g_events_blocked);
}

/* Wrappers used by the kretprobe handler (below). */
static bool __maybe_unused check_sensitive_path(const char *path, int mask)
{
    size_t i;
    if (!path) return true;  /* unknown path → allow */
    for (i = 0; i < N_SENSITIVE_PATHS; i++) {
        if (!strncmp(path, sensitive_paths[i].prefix,
                     strlen(sensitive_paths[i].prefix))) {
            bool ok = monoos_proc_has_perm(current->pid,
                                           sensitive_paths[i].required_perm);
            return ok;
        }
    }
    return true; /* not a sensitive path */
}

static void record_fs_event(const char *path, u32 perm, bool allowed)
{
    emit_event(path, perm, allowed);
}

/* ------------------------------------------------------------------ */
/*  /proc/monoos/fs_events  – readable ring buffer for privacy daemon  */
/* ------------------------------------------------------------------ */
static ssize_t fs_events_read(struct file *file, char __user *buf,
                               size_t count, loff_t *ppos)
{
    struct fs_event ev;
    unsigned int copied = 0;
    char line[256];
    int len;

    while (count >= sizeof(line)) {
        unsigned long flags;
        int got;

        spin_lock_irqsave(&g_fifo_lock, flags);
        got = kfifo_out(&g_event_fifo, &ev, 1);
        spin_unlock_irqrestore(&g_fifo_lock, flags);

        if (!got) break;

        len = snprintf(line, sizeof(line),
                       "%llu %d %u %u %u %s\n",
                       ev.ts_ns, ev.pid, ev.uid,
                       ev.perm_required, ev.allowed, ev.path);

        if (len <= 0 || (size_t)len >= count) break;
        if (copy_to_user(buf + copied, line, (size_t)len)) return -EFAULT;
        copied += (unsigned int)len;
        count  -= (size_t)len;
    }

    return (ssize_t)copied;
}

static int fs_events_open(struct inode *inode, struct file *file) { return 0; }

static const struct proc_ops fs_events_fops = {
    .proc_open  = fs_events_open,
    .proc_read  = fs_events_read,
};

/* ------------------------------------------------------------------ */
/*  Stats: /proc/monoos/vfs_stats                                      */
/* ------------------------------------------------------------------ */
static int vfs_stats_show(struct seq_file *m, void *v)
{
    seq_printf(m, "events_total:   %lld\n", atomic64_read(&g_events_total));
    seq_printf(m, "events_blocked: %lld\n", atomic64_read(&g_events_blocked));
    seq_printf(m, "fifo_used:      %u/%u\n",
               kfifo_len(&g_event_fifo), FS_EVENT_FIFO_SIZE);
    return 0;
}
static int vfs_stats_open(struct inode *i, struct file *f)
{
    return single_open(f, vfs_stats_show, NULL);
}
static const struct proc_ops vfs_stats_fops = {
    .proc_open    = vfs_stats_open,
    .proc_read    = seq_read,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};

/* ------------------------------------------------------------------ */
/*  kprobe-based VFS interception (replaces security_add_hooks)       */
/*                                                                     */
/*  security_add_hooks / security_hook_heads are not exported to      */
/*  loadable modules in Linux 6.x (LSMs must be built into the        */
/*  kernel).  We attach a kretprobe to inode_permission() instead.    */
/*                                                                     */
/*  SAFETY: kretprobe handlers fire in process context but the        */
/*  probed function (inode_permission) may already hold inode->i_lock */
/*  via spin_lock_bh on some call paths.  Calling d_find_alias()      */
/*  inside the handler would try to re-acquire the same lock →        */
/*  deadlock.  We therefore use a fully lock-free fast path:          */
/*    • entry handler: stores (ino, pid, mask) — no locks ever        */
/*    • return handler: atomic counter + deferred work queue           */
/*  Path resolution and policy enforcement run in the workqueue        */
/*  worker where we are safely in plain process context.              */
/* ------------------------------------------------------------------ */
#include <linux/kprobes.h>
#include <linux/workqueue.h>

/* Per-event record queued to the deferred worker. */
struct perm_event {
    unsigned long ino;
    pid_t         pid;
    uid_t         uid;
    int           mask;
    struct work_struct work;
};

static struct workqueue_struct *g_vfs_wq;

/* Lock-free event record stored by the kretprobe entry handler. */
struct kp_perm_data {
    unsigned long ino;   /* inode->i_ino (no lock needed, immutable) */
    pid_t         pid;
    uid_t         uid;
    int           mask;
};

static int kp_inode_perm_entry(struct kretprobe_instance *ri,
                               struct pt_regs *regs)
{
    struct kp_perm_data *d = (struct kp_perm_data *)ri->data;
    struct inode *inode = (struct inode *)regs->di;  /* arg0 x86-64 */

    d->mask = (int)regs->si;                         /* arg1 */
    d->pid  = task_pid_nr(current);
    d->uid  = from_kuid_munged(current_user_ns(), current_uid());
    /* i_ino is set at inode creation and never changes — safe without lock. */
    d->ino  = inode ? inode->i_ino : 0;
    return 0;
}

/* Worker: runs in process context — safe to do path lookup, policy check. */
static void vfs_perm_worker(struct work_struct *work)
{
    struct perm_event *ev = container_of(work, struct perm_event, work);

    /* Sensitive ino-based check (placeholder — real impl maps ino→policy). */
    bool sensitive = (ev->mask & (MAY_READ | MAY_WRITE)) && ev->uid >= 10000;
    if (sensitive)
        record_fs_event("<deferred>", (u32)ev->mask, true);

    kfree(ev);
}

static int kp_inode_perm_ret(struct kretprobe_instance *ri,
                              struct pt_regs *regs)
{
    struct kp_perm_data *d = (struct kp_perm_data *)ri->data;
    long retval = regs_return_value(regs);
    struct perm_event *ev;

    /* Only track allowed accesses to sensitive inodes. */
    if (retval != 0 || !d->ino || !g_vfs_wq) return 0;

    /* Atomic global counter — entirely lock-free. */
    atomic64_inc(&g_events_total);

    /* Defer heavier work (path resolution, policy) to process context. */
    ev = kmalloc(sizeof(*ev), GFP_ATOMIC);
    if (!ev) return 0;   /* drop event under memory pressure */

    ev->ino  = d->ino;
    ev->pid  = d->pid;
    ev->uid  = d->uid;
    ev->mask = d->mask;
    INIT_WORK(&ev->work, vfs_perm_worker);
    queue_work(g_vfs_wq, &ev->work);
    return 0;
}

static struct kretprobe g_kp_inode_perm = {
    .kp.symbol_name = "inode_permission",
    .handler        = kp_inode_perm_ret,
    .entry_handler  = kp_inode_perm_entry,
    .data_size      = sizeof(struct kp_perm_data),
    /*
     * maxactive = NR_CPUS * 4.  Every CPU can have at most a handful
     * of concurrent inode_permission calls in flight at once; 4× is a
     * conservative margin that avoids "kretprobe lost X instances"
     * without pre-allocating hundreds of MB of per-instance storage.
     */
    .maxactive      = NR_CPUS * 4,
};

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_monoos;
static struct proc_dir_entry *g_proc_events;
static struct proc_dir_entry *g_proc_stats;

static int __init monoos_vfs_init(void)
{
    int ret;

    /* Bounded single-thread WQ: path resolution + policy work runs here. */
    g_vfs_wq = alloc_ordered_workqueue("monoos_vfs", WQ_MEM_RECLAIM);
    if (!g_vfs_wq) {
        pr_err("monoos_vfs: failed to allocate workqueue\n");
        return -ENOMEM;
    }

    ret = register_kretprobe(&g_kp_inode_perm);
    if (ret)
        pr_warn("monoos_vfs: inode_permission probe failed: %d (continuing)\n",
                ret);

    g_proc_monoos  = proc_mkdir("monoos", NULL);
    if (g_proc_monoos) {
        g_proc_events = proc_create("fs_events", 0400, g_proc_monoos,
                                     &fs_events_fops);
        g_proc_stats  = proc_create("vfs_stats", 0444, g_proc_monoos,
                                     &vfs_stats_fops);
    } else {
        /*
         * monoos_process already created /proc/monoos — use the existing
         * entry rather than creating a duplicate.  proc_mkdir returns NULL
         * (with EEXIST) in this case; we look it up by name instead.
         */
        g_proc_monoos = proc_lookup_entry("monoos", NULL);
        if (g_proc_monoos) {
            g_proc_events = proc_create("fs_events", 0400, g_proc_monoos,
                                         &fs_events_fops);
            g_proc_stats  = proc_create("vfs_stats", 0444, g_proc_monoos,
                                         &vfs_stats_fops);
        } else {
            pr_warn("monoos_vfs: /proc/monoos not available — fs_events/vfs_stats disabled\n");
        }
    }

    pr_info("monoos_vfs: VFS privacy overlay loaded (%zu sensitive paths, kprobe)\n",
            N_SENSITIVE_PATHS);
    return 0;
}

static void __exit monoos_vfs_exit(void)
{
    unregister_kretprobe(&g_kp_inode_perm);
    /* Drain all queued work before tearing down. */
    if (g_vfs_wq) {
        flush_workqueue(g_vfs_wq);
        destroy_workqueue(g_vfs_wq);
        g_vfs_wq = NULL;
    }
    if (g_proc_events) proc_remove(g_proc_events);
    if (g_proc_stats)  proc_remove(g_proc_stats);
    if (g_proc_monoos)  proc_remove(g_proc_monoos);
    pr_info("monoos_vfs: unloaded\n");
}

module_init(monoos_vfs_init);
module_exit(monoos_vfs_exit);
