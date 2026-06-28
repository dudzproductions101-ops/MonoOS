/*
 * syscalls.c – MonoOS custom syscall extensions (Linux kernel module)
 *
 * Adds three syscalls via the kernel's syscall table patching mechanism:
 *
 *   sys_monoos_perm_check(pid, perm)   – query a process's runtime permission.
 *   sys_monoos_perm_set(pid, perm, val)– grant/revoke a permission (privileged).
 *   sys_monoos_privacy_stat(buf, len)  – dump the privacy dashboard snapshot.
 *
 * On ARM64 we inject these into the __NR_unused* slots (>=400) in the
 * compat and native syscall tables.  On x86-64 we use the same range.
 *
 * Build: add CONFIG_MONOOS_SYSCALLS=y to .config, or build as a module
 * and patch the table via kallsyms_lookup_name().
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/uaccess.h>
#include <linux/slab.h>
#include <linux/pid.h>
#include <linux/sched.h>
#include <linux/atomic.h>
#include <linux/spinlock.h>
#include <linux/string.h>
#include <linux/capability.h>   /* capable(CAP_SYS_ADMIN) */
#include "monoos_process.h"     /* monoos_proc_has_perm, monoos_proc_grant_perm */

/*
 * kallsyms_lookup_name was un-exported from module use in Linux 5.7.
 * The standard workaround for out-of-tree modules is to resolve it once
 * at load time via a kprobe placed on a known exported symbol, then read
 * the kprobe's address and scan forward.  A simpler and equally portable
 * approach supported by all kernels ≥ 5.7 is to use kprobe on the
 * kallsyms_lookup_name function itself – kprobe registration fills in
 * kp.addr before calling the handler, so we can capture it immediately.
 */
MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS custom syscall extensions via /dev/monoos ioctl");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Permission constants (mirrored in userspace sdk/api/monoos_perm.h) */
/* ------------------------------------------------------------------ */
#define MONOOS_PERM_CAMERA      0x0001U
#define MONOOS_PERM_MIC         0x0002U
#define MONOOS_PERM_LOCATION    0x0004U
#define MONOOS_PERM_CONTACTS    0x0008U
#define MONOOS_PERM_STORAGE     0x0010U
#define MONOOS_PERM_PHONE       0x0020U
#define MONOOS_PERM_BLUETOOTH   0x0040U
#define MONOOS_PERM_NFC         0x0080U
#define MONOOS_PERM_SENSORS     0x0100U
#define MONOOS_PERM_ALL         0x01FFU

/* UID that may call sys_monoos_perm_set (the permission service). */
#define MONOOS_PERM_SERVICE_UID  1000U

/* ------------------------------------------------------------------ */
/*  Shared permission table (same data as monoos_process module)       */
/*  We access it via a forward-declared extern symbol exported there.  */
/* ------------------------------------------------------------------ */
extern int  monoos_proc_grant_perm(pid_t pid, u32 perm_bit);
extern int  monoos_proc_revoke_perm(pid_t pid, u32 perm_bit);
extern bool monoos_proc_has_perm(pid_t pid, u32 perm_bit);

/* ------------------------------------------------------------------ */
/*  Privacy snapshot structure (written to userspace by privacy_stat)  */
/* ------------------------------------------------------------------ */
struct monoos_privacy_snap {
    __u32 camera_pid;       /* PID currently using camera, 0=none      */
    __u32 mic_pid;          /* PID currently using microphone          */
    __u32 location_pid;     /* PID currently accessing GPS             */
    __u32 active_perm_mask; /* OR of all active permission bits        */
    __u64 snapshot_ns;      /* ktime_get_ns() when snapshot was taken  */
    __u32 reserved[4];
} __attribute__((packed));

/* Global snapshot, updated by the permission service. */
static struct monoos_privacy_snap g_snap;
static DEFINE_SPINLOCK(g_snap_lock);

static atomic_t g_perm_check_calls = ATOMIC_INIT(0);
static atomic_t g_perm_set_calls   = ATOMIC_INIT(0);

/* ------------------------------------------------------------------ */
/*  sys_monoos_perm_check                                               */
/*                                                                     */
/*  Query whether process @pid currently holds permission @perm.       */
/*  Returns 1 if granted, 0 if denied, negative errno on error.        */
/* ------------------------------------------------------------------ */
static long sys_monoos_perm_check(pid_t pid, u32 perm)
{
    atomic_inc(&g_perm_check_calls);

    if (perm == 0 || (perm & ~MONOOS_PERM_ALL))
        return -EINVAL;
    if (pid <= 0)
        return -ESRCH;

    return monoos_proc_has_perm(pid, perm) ? 1 : 0;
}

/* ------------------------------------------------------------------ */
/*  sys_monoos_perm_set                                                 */
/*                                                                     */
/*  Grant (@val=1) or revoke (@val=0) permission @perm for @pid.      */
/*  Only callable by the permission service (UID 1000).               */
/* ------------------------------------------------------------------ */
static long sys_monoos_perm_set(pid_t pid, u32 perm, int val)
{
    uid_t caller_uid;

    atomic_inc(&g_perm_set_calls);

    caller_uid = from_kuid_munged(current_user_ns(), current_uid());
    if (caller_uid != MONOOS_PERM_SERVICE_UID && !capable(CAP_SYS_ADMIN))
        return -EPERM;

    if (perm == 0 || (perm & ~MONOOS_PERM_ALL))
        return -EINVAL;
    if (pid <= 0)
        return -ESRCH;

    if (val)
        return monoos_proc_grant_perm(pid, perm);
    else
        return monoos_proc_revoke_perm(pid, perm);
}

/* ------------------------------------------------------------------ */
/*  sys_monoos_privacy_stat                                             */
/*                                                                     */
/*  Copy the current privacy snapshot to the @buf/__user pointer.     */
/*  @len must be >= sizeof(struct monoos_privacy_snap).                 */
/* ------------------------------------------------------------------ */
static long sys_monoos_privacy_stat(void __user *buf, size_t len)
{
    struct monoos_privacy_snap snap;

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
/* ------------------------------------------------------------------ */
/*  /dev/monoos character device                                       */
/*                                                                     */
/*  CONFIG_STRICT_KERNEL_RWX (enabled in all modern Ubuntu kernels)    */
/*  makes the syscall table page read-only at the page-table level,    */
/*  which cannot be bypassed by clearing CR0.WP.  Patching the syscall */
/*  table at runtime is therefore impossible on a stock vendor kernel.  */
/*                                                                     */
/*  The correct production approach is a character device with ioctl.  */
/*  This is how Android vendor HALs expose custom kernel interfaces to  */
/*  userspace — the same design MonoOS uses here.                       */
/*                                                                     */
/*  Userspace calls:                                                    */
/*    fd = open("/dev/monoos", O_RDWR)                                 */
/*    ioctl(fd, MONOOS_IOC_PERM_CHECK, &req)                           */
/*    ioctl(fd, MONOOS_IOC_PERM_SET,   &req)                           */
/*    ioctl(fd, MONOOS_IOC_PRIVACY_STAT, &snap)                        */
/* ------------------------------------------------------------------ */
#include <linux/cdev.h>
#include <linux/ioctl.h>
#include <linux/fs.h>

/* ioctl command numbers */
#define MONOOS_IOC_MAGIC     'm'
#define MONOOS_IOC_PERM_CHECK    _IOR(MONOOS_IOC_MAGIC, 1, struct monoos_perm_req)
#define MONOOS_IOC_PERM_SET      _IOW(MONOOS_IOC_MAGIC, 2, struct monoos_perm_req)
#define MONOOS_IOC_PRIVACY_STAT  _IOR(MONOOS_IOC_MAGIC, 3, struct monoos_privacy_snap)

struct monoos_perm_req {
    pid_t  pid;
    u32    perm;
    int    val;    /* for SET: 1=grant, 0=revoke */
    int    result; /* filled by kernel on CHECK */
};

static dev_t           g_devno;
static struct cdev     g_cdev;
static struct class   *g_class;
static struct device  *g_device;

static long monoos_dev_ioctl(struct file *file, unsigned int cmd,
                              unsigned long arg)
{
    void __user *uarg = (void __user *)arg;

    switch (cmd) {

    case MONOOS_IOC_PERM_CHECK: {
        struct monoos_perm_req req;
        if (copy_from_user(&req, uarg, sizeof(req)))
            return -EFAULT;
        req.result = (int)sys_monoos_perm_check(req.pid, req.perm);
        if (copy_to_user(uarg, &req, sizeof(req)))
            return -EFAULT;
        return 0;
    }

    case MONOOS_IOC_PERM_SET: {
        struct monoos_perm_req req;
        if (!capable(CAP_SYS_ADMIN))
            return -EPERM;
        if (copy_from_user(&req, uarg, sizeof(req)))
            return -EFAULT;
        return sys_monoos_perm_set(req.pid, req.perm, req.val);
    }

    case MONOOS_IOC_PRIVACY_STAT: {
        struct monoos_privacy_snap snap;
        long ret = sys_monoos_privacy_stat(NULL, 0);
        spin_lock(&g_snap_lock);
        snap = g_snap;
        spin_unlock(&g_snap_lock);
        if (copy_to_user(uarg, &snap, sizeof(snap)))
            return -EFAULT;
        return ret;
    }

    default:
        return -ENOTTY;
    }
}

static const struct file_operations g_monoos_fops = {
    .owner          = THIS_MODULE,
    .unlocked_ioctl = monoos_dev_ioctl,
};

static int register_monoos_device(void)
{
    int ret;

    ret = alloc_chrdev_region(&g_devno, 0, 1, "monoos");
    if (ret < 0) {
        pr_err("monoos_syscalls: alloc_chrdev_region failed: %d\n", ret);
        return ret;
    }

    cdev_init(&g_cdev, &g_monoos_fops);
    g_cdev.owner = THIS_MODULE;
    ret = cdev_add(&g_cdev, g_devno, 1);
    if (ret) {
        pr_err("monoos_syscalls: cdev_add failed: %d\n", ret);
        unregister_chrdev_region(g_devno, 1);
        return ret;
    }

    g_class = class_create("monoos");
    if (IS_ERR(g_class)) {
        ret = PTR_ERR(g_class);
        cdev_del(&g_cdev);
        unregister_chrdev_region(g_devno, 1);
        return ret;
    }

    g_device = device_create(g_class, NULL, g_devno, NULL, "monoos");
    if (IS_ERR(g_device)) {
        ret = PTR_ERR(g_device);
        class_destroy(g_class);
        cdev_del(&g_cdev);
        unregister_chrdev_region(g_devno, 1);
        return ret;
    }

    pr_info("monoos_syscalls: /dev/monoos created (major=%d)\n",
            MAJOR(g_devno));
    return 0;
}

static void unregister_monoos_device(void)
{
    device_destroy(g_class, g_devno);
    class_destroy(g_class);
    cdev_del(&g_cdev);
    unregister_chrdev_region(g_devno, 1);
}

static int __init monoos_syscalls_init(void)
{
    int ret = register_monoos_device();
    if (ret) return ret;
    pr_info("monoos_syscalls: loaded (ioctl-based, /dev/monoos)\n");
    return 0;
}

static void __exit monoos_syscalls_exit(void)
{
    unregister_monoos_device();
    pr_info("monoos_syscalls: unloaded (perm_check=%d perm_set=%d)\n",
            atomic_read(&g_perm_check_calls),
            atomic_read(&g_perm_set_calls));
}

module_init(monoos_syscalls_init);
module_exit(monoos_syscalls_exit);
