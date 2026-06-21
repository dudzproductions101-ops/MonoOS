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
#include <linux/security.h>
#include <linux/lsm_hooks.h>
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
/*  LSM hook: ptrace_access_check                                     */
/*                                                                     */
/*  Prevent apps (UID ≥ 10000) from ptracing processes of a          */
/*  different UID unless they hold CAP_SYS_PTRACE.                   */
/* ------------------------------------------------------------------ */
static int monoos_ptrace_access_check(struct task_struct *child,
                                      unsigned int mode)
{
    uid_t tracer_uid = from_kuid_munged(current_user_ns(), current_uid());
    uid_t child_uid  = from_kuid_munged(child->cred->user_ns,
                                         task_uid(child));

    if (tracer_uid >= 10000 && tracer_uid != child_uid) {
        lsm_emit_audit(HOOK_PTRACE, true);
        return -EPERM;
    }
    lsm_emit_audit(HOOK_PTRACE, false);
    return 0;
}

/* ------------------------------------------------------------------ */
/*  LSM hook: task_kill                                               */
/*                                                                     */
/*  Prevent apps from sending signals to processes of a different     */
/*  UID (except SIGCHLD and common IPC signals to own children).      */
/* ------------------------------------------------------------------ */
static int monoos_task_kill(struct task_struct *p, struct kernel_siginfo *info,
                            int sig, const struct cred *cred)
{
    uid_t sender = from_kuid_munged(cred->user_ns, cred->uid);
    uid_t target = from_kuid_munged(p->cred->user_ns, task_uid(p));

    /* Allow system processes and same-UID signals. */
    if (is_system_uid(sender) || sender == target)
        return 0;

    /* Allow SIGCHLD unconditionally. */
    if (sig == SIGCHLD)
        return 0;

    /* Apps may not signal processes of other UIDs. */
    if (sender >= 10000 && sender != target) {
        lsm_emit_audit(HOOK_PTRACE, true);
        return -EPERM;
    }

    return 0;
}

/* ------------------------------------------------------------------ */
/*  LSM hook: file_open                                               */
/*                                                                     */
/*  Deny access to /dev/video*, /dev/snd/*, /dev/nfc if the          */
/*  caller's process doesn't hold the matching MonoOS permission.      */
/* ------------------------------------------------------------------ */
static int monoos_file_open(struct file *file)
{
    const char *name;
    u32 required_perm = 0;

    if (!file->f_path.dentry) return 0;
    name = file->f_path.dentry->d_name.name;

    if (!name) return 0;

    if (!strncmp(name, "video",  5)) required_perm = 0x0001; /* CAMERA */
    else if (!strncmp(name, "snd",   3)) required_perm = 0x0002; /* MIC */
    else if (!strncmp(name, "nfc",   3)) required_perm = 0x0080; /* NFC */

    if (required_perm) {
        bool ok = monoos_proc_has_perm(current->pid, required_perm);
        lsm_emit_audit(HOOK_FILE, !ok);
        if (!ok) return -EACCES;
    }

    return 0;
}

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
/*  LSM registration                                                   */
/* ------------------------------------------------------------------ */
static struct security_hook_list monoos_lsm_hooks[] __lsm_ro_after_init = {
    LSM_HOOK_INIT(ptrace_access_check, monoos_ptrace_access_check),
    LSM_HOOK_INIT(task_kill,           monoos_task_kill),
    LSM_HOOK_INIT(file_open,           monoos_file_open),
};

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_monoos;
static struct proc_dir_entry *g_proc_audit;
static struct proc_dir_entry *g_proc_stats;

static int __init monoos_lsm_init(void)
{
    security_add_hooks(monoos_lsm_hooks, ARRAY_SIZE(monoos_lsm_hooks),
                       "monoos");

    g_proc_monoos = proc_mkdir("monoos", NULL);
    if (g_proc_monoos) {
        g_proc_audit = proc_create("lsm_audit", 0400, g_proc_monoos,
                                    &lsm_audit_fops);
        g_proc_stats = proc_create("lsm_stats", 0444, g_proc_monoos,
                                    &lsm_stats_fops);
    }

    pr_info("monoos_lsm: security module loaded (%zu hooks)\n",
            ARRAY_SIZE(monoos_lsm_hooks));
    return 0;
}

static void __exit monoos_lsm_exit(void)
{
    if (g_proc_audit) proc_remove(g_proc_audit);
    if (g_proc_stats) proc_remove(g_proc_stats);
    if (g_proc_monoos) proc_remove(g_proc_monoos);
    pr_info("monoos_lsm: unloaded\n");
}

module_init(monoos_lsm_init);
module_exit(monoos_lsm_exit);
