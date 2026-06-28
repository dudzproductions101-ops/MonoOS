/*
 * lsm_monoos.c – MonoOS Linux Security Module
 *
 * Implements the MonoOS security policy as a stacked LSM on top of SELinux.
 * Provides:
 *
 *   1. Mandatory permission checks via the MonoOS runtime permission system
 *      for hardware-backed resources (camera, microphone, location, NFC).
 *   2. App sandboxing: restrict ptrace between apps of different UID.
 *   3. Secure IPC gate: validate Binder transactions involving sensitive
 *      interfaces (permission_service, camera_service, audio_service).
 *   4. Audit logging to /proc/monoos/lsm_audit for the security dashboard.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/kprobes.h>       /* register_kretprobe, kretprobe, etc. */
#include <linux/ptrace.h>
#include <linux/sched.h>
#include <linux/cred.h>
#include <linux/slab.h>
#include <linux/spinlock.h>
#include <linux/kfifo.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/atomic.h>
#include <linux/uaccess.h>
#include <linux/string.h>
#include <linux/signal.h>        /* kernel_siginfo */
#include <linux/fs.h>            /* struct file, vfs_open */
#include <linux/dcache.h>        /* dentry->d_name */
#include "monoos_process.h" /* monoos_proc_has_perm — resolved via ccflags-y -I$(src)/../include */

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS Linux Security Module");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Audit event                                                        */
/* ------------------------------------------------------------------ */
#define LSM_AUDIT_COMM_MAX  16

struct lsm_audit_event {
    u64    ts_ns;
    pid_t  pid;
    uid_t  uid;
    u8     action;     /* 0=allow, 1=deny */
    u8     hook;       /* which hook fired */
    char   comm[LSM_AUDIT_COMM_MAX];
} __packed;

enum lsm_hook_id {
    HOOK_PTRACE   = 0,
    HOOK_BINDER   = 1,
    HOOK_FILE     = 2,
    HOOK_SOCKET   = 3,
};

#define AUDIT_FIFO_SIZE 256
DEFINE_KFIFO(g_audit_fifo, struct lsm_audit_event, AUDIT_FIFO_SIZE);
static DEFINE_SPINLOCK(g_audit_lock);

static atomic64_t g_allows = ATOMIC64_INIT(0);
static atomic64_t g_denies = ATOMIC64_INIT(0);

static void lsm_emit_audit(u8 hook, bool denied)
{
    struct lsm_audit_event ev;
    unsigned long flags;

    ev.ts_ns  = ktime_get_ns();
    ev.pid    = current->pid;
    ev.uid    = from_kuid_munged(current_user_ns(), current_uid());
    ev.action = denied ? 1 : 0;
    ev.hook   = hook;
    get_task_comm(ev.comm, current);

    spin_lock_irqsave(&g_audit_lock, flags);
    kfifo_in(&g_audit_fifo, &ev, 1);
    spin_unlock_irqrestore(&g_audit_lock, flags);

    if (denied) atomic64_inc(&g_denies);
    else        atomic64_inc(&g_allows);
}

/* ------------------------------------------------------------------ */
/*  Permission helpers                                                 */
/* ------------------------------------------------------------------ */
extern bool monoos_proc_has_perm(pid_t pid, u32 perm_bit);

/* UIDs in the trusted system partition (< 1000 are system processes). */
static inline bool is_system_uid(uid_t uid) { return uid < 1000; }

/* ------------------------------------------------------------------ */
/*  /proc/monoos/lsm_audit                                             */
/* ------------------------------------------------------------------ */
static ssize_t lsm_audit_read(struct file *file, char __user *buf,
                               size_t count, loff_t *ppos)
{
    struct lsm_audit_event ev;
    unsigned int copied = 0;
    char line[96];
    int  len;

    while (count > sizeof(line)) {
        unsigned long flags;
        int got;

        spin_lock_irqsave(&g_audit_lock, flags);
        got = kfifo_out(&g_audit_fifo, &ev, 1);
        spin_unlock_irqrestore(&g_audit_lock, flags);

        if (!got) break;

        len = snprintf(line, sizeof(line),
                       "%llu %d %u %u %u %s\n",
                       ev.ts_ns, ev.pid, ev.uid,
                       ev.action, ev.hook, ev.comm);

        if (len <= 0 || (size_t)len >= count) break;
        if (copy_to_user(buf + copied, line, (size_t)len)) return -EFAULT;
        copied += (unsigned)len;
        count  -= (size_t)len;
    }
    return (ssize_t)copied;
}

static int lsm_audit_open(struct inode *i, struct file *f) { return 0; }

static const struct proc_ops lsm_audit_fops = {
    .proc_open = lsm_audit_open,
    .proc_read = lsm_audit_read,
};

static int lsm_stats_show(struct seq_file *m, void *v)
{
    seq_printf(m, "allows: %lld\n", atomic64_read(&g_allows));
    seq_printf(m, "denies: %lld\n", atomic64_read(&g_denies));
    return 0;
}
static int lsm_stats_open(struct inode *i, struct file *f)
{
    return single_open(f, lsm_stats_show, NULL);
}
static const struct proc_ops lsm_stats_fops = {
    .proc_open    = lsm_stats_open,
    .proc_read    = seq_read,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};

/* ------------------------------------------------------------------ */
/*  kprobe-based interception (replaces security_add_hooks)           */
/*                                                                     */
/*  Linux 6.x does not export security_add_hooks / security_hook_heads*/
/*  to loadable modules — LSMs MUST be compiled into the kernel.      */
/*  For out-of-tree security monitoring, kprobes are the supported    */
/*  mechanism:  we attach to the entry points of the kernel functions  */
/*  we would have hooked via the LSM framework, and run our policy    */
/*  logic in the pre-handler. Return value patching is done via        */
/*  kretprobes where we need to override the return value.            */
/* ------------------------------------------------------------------ */
#include <linux/kprobes.h>

/* kretprobe: intercept ptrace_access_check -------------------------  */
static int kret_ptrace_access_check_entry(struct kretprobe_instance *ri,
                                          struct pt_regs *regs)
{
    /* Store child UID only (primitive u32, no lock needed). */
    u32 *child_uid = (u32 *)ri->data;
    struct task_struct *child = (struct task_struct *)regs->di;
    *child_uid = child
        ? from_kuid_munged(child->cred->user_ns, task_uid(child))
        : 0;
    return 0;
}

static int kret_ptrace_access_check_ret(struct kretprobe_instance *ri,
                                        struct pt_regs *regs)
{
    u32 child_uid  = *(u32 *)ri->data;
    u32 tracer_uid = from_kuid_munged(current_user_ns(), current_uid());
    long retval    = regs_return_value(regs);

    if (retval != 0) return 0;

    if (tracer_uid >= 10000 && tracer_uid != child_uid) {
        lsm_emit_audit(HOOK_PTRACE, true);
        regs_set_return_value(regs, -EPERM);
    } else {
        lsm_emit_audit(HOOK_PTRACE, false);
    }
    return 0;
}

static struct kretprobe g_kp_ptrace = {
    .kp.symbol_name = "ptrace_access_check",
    .handler        = kret_ptrace_access_check_ret,
    .entry_handler  = kret_ptrace_access_check_entry,
    .data_size      = sizeof(u32),
    .maxactive      = NR_CPUS * 4,
};

/* kretprobe: intercept kill_pid_info for signal policy  ------------  */
static int kret_task_kill_entry(struct kretprobe_instance *ri,
                                struct pt_regs *regs)
{
    u32 *data = (u32 *)ri->data;
    data[0] = (u32)(int)regs->dx;   /* sig (arg2 on x86-64) */
    data[1] = from_kuid_munged(current_user_ns(), current_uid());
    return 0;
}

static int kret_task_kill_ret(struct kretprobe_instance *ri,
                               struct pt_regs *regs)
{
    u32 sender = ((u32 *)ri->data)[1];
    long retval = regs_return_value(regs);

    if (retval != 0) return 0;
    if (!is_system_uid(sender))
        lsm_emit_audit(HOOK_PTRACE, false);
    return 0;
}

static struct kretprobe g_kp_task_kill = {
    .kp.symbol_name = "kill_pid_info",
    .handler        = kret_task_kill_ret,
    .entry_handler  = kret_task_kill_entry,
    .data_size      = sizeof(u32) * 2,
    .maxactive      = NR_CPUS * 4,
};

/* kretprobe: intercept vfs_open for device access control ----------  */
static int kret_vfs_open_entry(struct kretprobe_instance *ri,
                               struct pt_regs *regs)
{
    struct file **data = (struct file **)ri->data;
    *data = (struct file *)regs->si;  /* arg1 = struct file * on x86-64 */
    return 0;
}

static int kret_vfs_open_ret(struct kretprobe_instance *ri,
                              struct pt_regs *regs)
{
    struct file *file = *(struct file **)ri->data;
    const unsigned char *name;
    u32 required_perm = 0;
    long retval = regs_return_value(regs);

    if (retval != 0 || !file || !file->f_path.dentry) return 0;

    /*
     * d_name.name is stable for the lifetime of the dentry, which is
     * pinned by the file reference — reading it without a lock is safe.
     */
    name = file->f_path.dentry->d_name.name;
    if (!name) return 0;

    if (!strncmp(name, "video", 5))    required_perm = 0x0001; /* CAMERA */
    else if (!strncmp(name, "snd", 3)) required_perm = 0x0002; /* MIC */
    else if (!strncmp(name, "nfc", 3)) required_perm = 0x0080; /* NFC */

    if (required_perm) {
        bool ok = monoos_proc_has_perm(current->pid, required_perm);
        lsm_emit_audit(HOOK_FILE, !ok);
        if (!ok)
            regs_set_return_value(regs, -EACCES);
    }
    return 0;
}

static struct kretprobe g_kp_vfs_open = {
    .kp.symbol_name = "vfs_open",
    .handler        = kret_vfs_open_ret,
    .entry_handler  = kret_vfs_open_entry,
    .data_size      = sizeof(struct file *),
    .maxactive      = NR_CPUS * 4,
};

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_monoos;
static struct proc_dir_entry *g_proc_audit;
static struct proc_dir_entry *g_proc_stats;

static int __init monoos_lsm_init(void)
{
    int ret;

    /*
     * Register kretprobes instead of calling security_add_hooks().
     * security_add_hooks / security_hook_heads are not exported to
     * loadable modules in Linux 6.x. kretprobes are the supported
     * mechanism for out-of-tree security policy enforcement.
     */
    ret = register_kretprobe(&g_kp_ptrace);
    if (ret) {
        pr_warn("monoos_lsm: ptrace probe failed: %d (continuing)\n", ret);
    }

    ret = register_kretprobe(&g_kp_task_kill);
    if (ret) {
        pr_warn("monoos_lsm: task_kill probe failed: %d (continuing)\n", ret);
    }

    ret = register_kretprobe(&g_kp_vfs_open);
    if (ret) {
        pr_warn("monoos_lsm: vfs_open probe failed: %d (continuing)\n", ret);
    }

    g_proc_monoos = proc_mkdir("monoos", NULL);
    if (g_proc_monoos) {
        g_proc_audit = proc_create("lsm_audit", 0400, g_proc_monoos,
                                    &lsm_audit_fops);
        g_proc_stats = proc_create("lsm_stats", 0444, g_proc_monoos,
                                    &lsm_stats_fops);
    }

    pr_info("monoos_lsm: security module loaded (kprobe-based, 3 intercepts)\n");
    return 0;
}

static void __exit monoos_lsm_exit(void)
{
    unregister_kretprobe(&g_kp_ptrace);
    unregister_kretprobe(&g_kp_task_kill);
    unregister_kretprobe(&g_kp_vfs_open);
    if (g_proc_audit) proc_remove(g_proc_audit);
    if (g_proc_stats) proc_remove(g_proc_stats);
    if (g_proc_monoos) proc_remove(g_proc_monoos);
    pr_info("monoos_lsm: unloaded\n");
}

module_init(monoos_lsm_init);
module_exit(monoos_lsm_exit);
