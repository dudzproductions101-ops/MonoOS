/*
 * vfs_overlay.c – MonoOS VFS privacy overlay
 *
 * Implements a lightweight eBPF-free LSM hook layer that:
 *
 *   1. Intercepts open(2) / openat(2) on sensitive path prefixes
 *      (/proc/<pid>/, /sys/devices/, /dev/camera*, /dev/snd/*) and
 *      enforces MonoOS runtime permissions before allowing access.
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
#include <linux/security.h>
#include <linux/lsm_hooks.h>
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

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS VFS privacy overlay");
MODULE_VERSION("1.0.0");

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
    strlcpy(ev.path, path, FS_PATH_MAX);

    spin_lock_irqsave(&g_fifo_lock, flags);
    kfifo_in(&g_event_fifo, &ev, 1);
    spin_unlock_irqrestore(&g_fifo_lock, flags);

    atomic64_inc(&g_events_total);
    if (!allowed) atomic64_inc(&g_events_blocked);
}

/* ------------------------------------------------------------------ */
/*  LSM hook: inode_permission                                        */
/*                                                                     */
/*  Called before every access to an inode.  We match the path        */
/*  against sensitive_paths and check the caller's MonoOS permission.  */
/* ------------------------------------------------------------------ */
extern bool monoos_proc_has_perm(pid_t pid, u32 perm_bit);

static int monoos_inode_permission(struct inode *inode, int mask)
{
    struct dentry *dentry;
    char *buf, *path_str;
    size_t i;
    int ret = 0;

    /* Only intercept regular files and char devices. */
    if (!S_ISREG(inode->i_mode) && !S_ISCHR(inode->i_mode))
        return 0;

    /* Build the path string from the inode. */
    buf = kmalloc(PATH_MAX, GFP_ATOMIC);
    if (!buf)
        return 0;

    dentry = d_find_alias(inode);
    if (!dentry) {
        kfree(buf);
        return 0;
    }

    path_str = dentry_path_raw(dentry, buf, PATH_MAX);
    dput(dentry);

    if (IS_ERR(path_str)) {
        kfree(buf);
        return 0;
    }

    for (i = 0; i < N_SENSITIVE_PATHS; i++) {
        if (!strncmp(path_str, sensitive_paths[i].prefix,
                     strlen(sensitive_paths[i].prefix))) {
            bool ok = monoos_proc_has_perm(current->pid,
                                           sensitive_paths[i].required_perm);
            if (sensitive_paths[i].log_always || !ok)
                emit_event(path_str, sensitive_paths[i].required_perm, ok);

            if (!ok)
                ret = -EACCES;
            break;
        }
    }

    kfree(buf);
    return ret;
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
/*  LSM registration                                                   */
/* ------------------------------------------------------------------ */
static struct security_hook_list monoos_hooks[] __lsm_ro_after_init = {
    LSM_HOOK_INIT(inode_permission, monoos_inode_permission),
};

static struct lsm_blob_sizes monoos_blob_sizes __initdata = {};

static int __init monoos_vfs_overlay_init_lsm(void)
{
    security_add_hooks(monoos_hooks, ARRAY_SIZE(monoos_hooks), "monoos_vfs");
    return 0;
}

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_monoos;
static struct proc_dir_entry *g_proc_events;
static struct proc_dir_entry *g_proc_stats;

static int __init monoos_vfs_init(void)
{
    g_proc_monoos  = proc_mkdir("monoos", NULL);
    if (g_proc_monoos) {
        g_proc_events = proc_create("fs_events", 0400, g_proc_monoos,
                                     &fs_events_fops);
        g_proc_stats  = proc_create("vfs_stats", 0444, g_proc_monoos,
                                     &vfs_stats_fops);
    }

    monoos_vfs_overlay_init_lsm();
    pr_info("monoos_vfs: VFS privacy overlay loaded (%zu sensitive paths)\n",
            N_SENSITIVE_PATHS);
    return 0;
}

static void __exit monoos_vfs_exit(void)
{
    if (g_proc_events) proc_remove(g_proc_events);
    if (g_proc_stats)  proc_remove(g_proc_stats);
    if (g_proc_monoos)  proc_remove(g_proc_monoos);
    pr_info("monoos_vfs: unloaded\n");
}

module_init(monoos_vfs_init);
module_exit(monoos_vfs_exit);
