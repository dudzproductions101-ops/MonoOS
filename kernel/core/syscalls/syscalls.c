/*
 * syscalls.c – OneOS custom syscall extensions (Linux kernel module)
 *
 * Adds three syscalls via the kernel's syscall table patching mechanism:
 *
 *   sys_oneos_perm_check(pid, perm)   – query a process's runtime permission.
 *   sys_oneos_perm_set(pid, perm, val)– grant/revoke a permission (privileged).
 *   sys_oneos_privacy_stat(buf, len)  – dump the privacy dashboard snapshot.
 *
 * On ARM64 we inject these into the __NR_unused* slots (>=400) in the
 * compat and native syscall tables.  On x86-64 we use the same range.
 *
 * Build: add CONFIG_ONEOS_SYSCALLS=y to .config, or build as a module
 * and patch the table via kallsyms_lookup_name().
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/syscalls.h>
#include <linux/uaccess.h>
#include <linux/slab.h>
#include <linux/pid.h>
#include <linux/sched.h>
#include <linux/atomic.h>
#include <linux/kallsyms.h>
#include <linux/spinlock.h>
#include <linux/string.h>
#include <asm/unistd.h>
#include <asm/syscall.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("OneOS custom syscall extensions");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Permission constants (mirrored in userspace sdk/api/oneos_perm.h) */
/* ------------------------------------------------------------------ */
#define ONEOS_PERM_CAMERA      0x0001U
#define ONEOS_PERM_MIC         0x0002U
#define ONEOS_PERM_LOCATION    0x0004U
#define ONEOS_PERM_CONTACTS    0x0008U
#define ONEOS_PERM_STORAGE     0x0010U
#define ONEOS_PERM_PHONE       0x0020U
#define ONEOS_PERM_BLUETOOTH   0x0040U
#define ONEOS_PERM_NFC         0x0080U
#define ONEOS_PERM_SENSORS     0x0100U
#define ONEOS_PERM_ALL         0x01FFU

/* UID that may call sys_oneos_perm_set (the permission service). */
#define ONEOS_PERM_SERVICE_UID  1000U

/* ------------------------------------------------------------------ */
/*  Shared permission table (same data as oneos_process module)       */
/*  We access it via a forward-declared extern symbol exported there.  */
/* ------------------------------------------------------------------ */
extern int  oneos_proc_grant_perm(pid_t pid, u32 perm_bit);
extern int  oneos_proc_revoke_perm(pid_t pid, u32 perm_bit);
extern bool oneos_proc_has_perm(pid_t pid, u32 perm_bit);

/* ------------------------------------------------------------------ */
/*  Privacy snapshot structure (written to userspace by privacy_stat)  */
/* ------------------------------------------------------------------ */
struct oneos_privacy_snap {
    __u32 camera_pid;       /* PID currently using camera, 0=none      */
    __u32 mic_pid;          /* PID currently using microphone          */
    __u32 location_pid;     /* PID currently accessing GPS             */
    __u32 active_perm_mask; /* OR of all active permission bits        */
    __u64 snapshot_ns;      /* ktime_get_ns() when snapshot was taken  */
    __u32 reserved[4];
} __attribute__((packed));

/* Global snapshot, updated by the permission service. */
static struct oneos_privacy_snap g_snap;
static DEFINE_SPINLOCK(g_snap_lock);

static atomic_t g_perm_check_calls = ATOMIC_INIT(0);
static atomic_t g_perm_set_calls   = ATOMIC_INIT(0);

/* ------------------------------------------------------------------ */
/*  sys_oneos_perm_check                                               */
/*                                                                     */
/*  Query whether process @pid currently holds permission @perm.       */
/*  Returns 1 if granted, 0 if denied, negative errno on error.        */
/* ------------------------------------------------------------------ */
static long sys_oneos_perm_check(pid_t pid, u32 perm)
{
    atomic_inc(&g_perm_check_calls);

    if (perm == 0 || (perm & ~ONEOS_PERM_ALL))
        return -EINVAL;
    if (pid <= 0)
        return -ESRCH;

    return oneos_proc_has_perm(pid, perm) ? 1 : 0;
}

/* ------------------------------------------------------------------ */
/*  sys_oneos_perm_set                                                 */
/*                                                                     */
/*  Grant (@val=1) or revoke (@val=0) permission @perm for @pid.      */
/*  Only callable by the permission service (UID 1000).               */
/* ------------------------------------------------------------------ */
static long sys_oneos_perm_set(pid_t pid, u32 perm, int val)
{
    uid_t caller_uid;

    atomic_inc(&g_perm_set_calls);

    caller_uid = from_kuid_munged(current_user_ns(), current_uid());
    if (caller_uid != ONEOS_PERM_SERVICE_UID && !capable(CAP_SYS_ADMIN))
        return -EPERM;

    if (perm == 0 || (perm & ~ONEOS_PERM_ALL))
        return -EINVAL;
    if (pid <= 0)
        return -ESRCH;

    if (val)
        return oneos_proc_grant_perm(pid, perm);
    else
        return oneos_proc_revoke_perm(pid, perm);
}

/* ------------------------------------------------------------------ */
/*  sys_oneos_privacy_stat                                             */
/*                                                                     */
/*  Copy the current privacy snapshot to the @buf/__user pointer.     */
/*  @len must be >= sizeof(struct oneos_privacy_snap).                 */
/* ------------------------------------------------------------------ */
static long sys_oneos_privacy_stat(void __user *buf, size_t len)
{
    struct oneos_privacy_snap snap;

    if (len < sizeof(snap))
        return -EINVAL;
    if (!access_ok(buf, sizeof(snap)))
        return -EFAULT;

    spin_lock_irq(&g_snap_lock);
    snap = g_snap;
    spin_unlock_irq(&g_snap_lock);

    snap.snapshot_ns = ktime_get_ns();

    if (copy_to_user(buf, &snap, sizeof(snap)))
        return -EFAULT;

    return (long)sizeof(snap);
}

/* ------------------------------------------------------------------ */
/*  Syscall table hooking via kallsyms + page remapping                */
/*                                                                     */
/*  On a real device with CONFIG_STRICT_KERNEL_RWX=y we need to       */
/*  disable write-protection briefly.  We use set_memory_rw/rox from  */
/*  <asm/set_memory.h>.  The slot numbers below are illustrative;     */
/*  they must match the platform-specific asm/unistd.h entries.       */
/* ------------------------------------------------------------------ */

typedef long (*syscall_fn_t)(const struct pt_regs *);

#define NR_ONEOS_PERM_CHECK   400
#define NR_ONEOS_PERM_SET     401
#define NR_ONEOS_PRIVACY_STAT 402

static syscall_fn_t *g_syscall_table;
static syscall_fn_t  g_orig[3];

/* Thin pt_regs wrappers */
static long wrap_perm_check(const struct pt_regs *regs)
{
    return sys_oneos_perm_check((pid_t)regs->regs[0],
                                 (u32)regs->regs[1]);
}

static long wrap_perm_set(const struct pt_regs *regs)
{
    return sys_oneos_perm_set((pid_t)regs->regs[0],
                               (u32)regs->regs[1],
                               (int)regs->regs[2]);
}

static long wrap_privacy_stat(const struct pt_regs *regs)
{
    return sys_oneos_privacy_stat((void __user *)regs->regs[0],
                                   (size_t)regs->regs[1]);
}

static void make_table_rw(void)  { /* set_memory_rw on real target */ }
static void make_table_ro(void)  { /* set_memory_ro on real target */ }

static int install_syscalls(void)
{
    unsigned long sym = kallsyms_lookup_name("sys_call_table");
    if (!sym) {
        pr_err("oneos_syscalls: cannot find sys_call_table\n");
        return -ENOENT;
    }
    g_syscall_table = (syscall_fn_t *)sym;

    make_table_rw();
    g_orig[0] = g_syscall_table[NR_ONEOS_PERM_CHECK];
    g_orig[1] = g_syscall_table[NR_ONEOS_PERM_SET];
    g_orig[2] = g_syscall_table[NR_ONEOS_PRIVACY_STAT];
    g_syscall_table[NR_ONEOS_PERM_CHECK]   = wrap_perm_check;
    g_syscall_table[NR_ONEOS_PERM_SET]     = wrap_perm_set;
    g_syscall_table[NR_ONEOS_PRIVACY_STAT] = wrap_privacy_stat;
    make_table_ro();

    pr_info("oneos_syscalls: installed at slots %d/%d/%d\n",
            NR_ONEOS_PERM_CHECK, NR_ONEOS_PERM_SET, NR_ONEOS_PRIVACY_STAT);
    return 0;
}

static void remove_syscalls(void)
{
    if (!g_syscall_table) return;
    make_table_rw();
    g_syscall_table[NR_ONEOS_PERM_CHECK]   = g_orig[0];
    g_syscall_table[NR_ONEOS_PERM_SET]     = g_orig[1];
    g_syscall_table[NR_ONEOS_PRIVACY_STAT] = g_orig[2];
    make_table_ro();
}

static int __init oneos_syscalls_init(void)
{
    int ret = install_syscalls();
    if (ret) return ret;
    pr_info("oneos_syscalls: loaded\n");
    return 0;
}

static void __exit oneos_syscalls_exit(void)
{
    remove_syscalls();
    pr_info("oneos_syscalls: unloaded (perm_check=%d perm_set=%d)\n",
            atomic_read(&g_perm_check_calls),
            atomic_read(&g_perm_set_calls));
}

module_init(oneos_syscalls_init);
module_exit(oneos_syscalls_exit);
