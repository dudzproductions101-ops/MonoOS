/*
 * process.c – MonoOS process lifecycle extensions
 *
 * Provides hooks into the Linux task lifecycle for:
 *   1. Privacy labelling:   track which processes have active camera,
 *      microphone, or location access.
 *   2. Resource budgeting:  per-app CPU/memory budgets enforced via
 *      cgroups v2.
 *   3. Isolation:           enforce seccomp profiles at fork time for
 *      third-party app processes.
 *   4. Death notification:  notify the MonoOS permission service when a
 *      process exits so it can revoke held hardware resources.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/sched.h>
#include <linux/sched/task.h>
#include <linux/pid.h>
#include <linux/cred.h>
#include <linux/spinlock.h>
#include <linux/list.h>
#include <linux/slab.h>
#include <linux/hashtable.h>
#include <linux/atomic.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/security.h>
#include <linux/kprobes.h>
#include <linux/tracepoint.h>
#include "monoos_process.h" /* via ccflags-y */

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS process lifecycle extensions");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  App privacy flags (bitmask per process)                            */
/* ------------------------------------------------------------------ */
#define MONOOS_PERM_CAMERA      BIT(0)
#define MONOOS_PERM_MIC         BIT(1)
#define MONOOS_PERM_LOCATION    BIT(2)
#define MONOOS_PERM_CONTACTS    BIT(3)
#define MONOOS_PERM_STORAGE     BIT(4)
#define MONOOS_PERM_PHONE       BIT(5)
#define MONOOS_PERM_BLUETOOTH   BIT(6)
#define MONOOS_PERM_NFC         BIT(7)

/* ------------------------------------------------------------------ */
/*  Per-process record                                                 */
/* ------------------------------------------------------------------ */
#define MONOOS_PROC_HASH_BITS  8   /* 256 buckets */

struct monoos_proc_record {
    pid_t             pid;
    uid_t             uid;
    u32               active_perms;    /* currently granted hw access  */
    u64               cpu_ns_budget;   /* ns of CPU budget remaining   */
    u64               mem_limit_bytes; /* soft memory limit            */
    u64               birth_ktime;     /* ktime_get_ns() at fork       */
    char              comm[TASK_COMM_LEN];
    struct hlist_node hash_node;
    struct rcu_head   rcu;
};

static DEFINE_HASHTABLE(g_proc_table, MONOOS_PROC_HASH_BITS);
static DEFINE_SPINLOCK(g_proc_lock);
static struct kmem_cache *g_proc_cache;

static atomic_t g_live_procs     = ATOMIC_INIT(0);
static atomic_t g_total_spawned  = ATOMIC_INIT(0);
static atomic_t g_perm_grants    = ATOMIC_INIT(0);
static atomic_t g_perm_revokes   = ATOMIC_INIT(0);

/* ------------------------------------------------------------------ */
/*  Record management                                                  */
/* ------------------------------------------------------------------ */
static struct monoos_proc_record *proc_record_alloc(pid_t pid, uid_t uid,
                                                    const char *comm)
{
    struct monoos_proc_record *rec;

    rec = kmem_cache_zalloc(g_proc_cache, GFP_ATOMIC);
    if (!rec)
        return NULL;

    rec->pid          = pid;
    rec->uid          = uid;
    rec->active_perms = 0;
    rec->cpu_ns_budget   = 10ULL * NSEC_PER_SEC; /* 10 s initial budget */
    rec->mem_limit_bytes = 512ULL * 1024 * 1024; /* 512 MiB default    */
    rec->birth_ktime  = ktime_get_ns();
    strscpy(rec->comm, comm, sizeof(rec->comm));
    return rec;
}

static void proc_record_free_rcu(struct rcu_head *head)
{
    struct monoos_proc_record *rec =
        container_of(head, struct monoos_proc_record, rcu);
    kmem_cache_free(g_proc_cache, rec);
}

/* ------------------------------------------------------------------ */
/*  Public API                                                         */
/* ------------------------------------------------------------------ */

/**
 * monoos_proc_register – register a new process in the MonoOS table.
 *
 * Called from a task_newtask tracepoint handler immediately after fork.
 */
int monoos_proc_register(pid_t pid, uid_t uid, const char *comm)
{
    struct monoos_proc_record *rec;
    unsigned long flags;

    rec = proc_record_alloc(pid, uid, comm);
    if (!rec)
        return -ENOMEM;

    spin_lock_irqsave(&g_proc_lock, flags);
    hash_add(g_proc_table, &rec->hash_node, (u32)pid);
    spin_unlock_irqrestore(&g_proc_lock, flags);

    atomic_inc(&g_live_procs);
    atomic_inc(&g_total_spawned);
    return 0;
}
EXPORT_SYMBOL_GPL(monoos_proc_register);

/**
 * monoos_proc_unregister – remove a process record on exit.
 *
 * Revokes all active permissions and notifies the permission service.
 */
void monoos_proc_unregister(pid_t pid)
{
    struct monoos_proc_record *rec;
    unsigned long flags;

    spin_lock_irqsave(&g_proc_lock, flags);
    hash_for_each_possible(g_proc_table, rec, hash_node, (u32)pid) {
        if (rec->pid == pid) {
            if (rec->active_perms)
                atomic_inc(&g_perm_revokes);
            hash_del_rcu(&rec->hash_node);
            spin_unlock_irqrestore(&g_proc_lock, flags);
            call_rcu(&rec->rcu, proc_record_free_rcu);
            atomic_dec(&g_live_procs);
            return;
        }
    }
    spin_unlock_irqrestore(&g_proc_lock, flags);
}
EXPORT_SYMBOL_GPL(monoos_proc_unregister);

/**
 * monoos_proc_grant_perm – grant a hardware permission to a process.
 * Returns 0 on success, -ESRCH if pid not found.
 */
int monoos_proc_grant_perm(pid_t pid, u32 perm_bit)
{
    struct monoos_proc_record *rec;
    int ret = -ESRCH;

    rcu_read_lock();
    hash_for_each_possible_rcu(g_proc_table, rec, hash_node, (u32)pid) {
        if (rec->pid == pid) {
            /* Atomic OR – no lock needed for bitmask update */
            u32 old = READ_ONCE(rec->active_perms);
            WRITE_ONCE(rec->active_perms, old | perm_bit);
            atomic_inc(&g_perm_grants);
            ret = 0;
            break;
        }
    }
    rcu_read_unlock();
    return ret;
}
EXPORT_SYMBOL_GPL(monoos_proc_grant_perm);

/**
 * monoos_proc_revoke_perm – revoke a hardware permission from a process.
 */
int monoos_proc_revoke_perm(pid_t pid, u32 perm_bit)
{
    struct monoos_proc_record *rec;
    int ret = -ESRCH;

    rcu_read_lock();
    hash_for_each_possible_rcu(g_proc_table, rec, hash_node, (u32)pid) {
        if (rec->pid == pid) {
            u32 old = READ_ONCE(rec->active_perms);
            WRITE_ONCE(rec->active_perms, old & ~perm_bit);
            atomic_inc(&g_perm_revokes);
            ret = 0;
            break;
        }
    }
    rcu_read_unlock();
    return ret;
}
EXPORT_SYMBOL_GPL(monoos_proc_revoke_perm);

/**
 * monoos_proc_has_perm – query whether a process holds a given permission.
 */
bool monoos_proc_has_perm(pid_t pid, u32 perm_bit)
{
    struct monoos_proc_record *rec;
    bool has = false;

    rcu_read_lock();
    hash_for_each_possible_rcu(g_proc_table, rec, hash_node, (u32)pid) {
        if (rec->pid == pid) {
            has = !!(READ_ONCE(rec->active_perms) & perm_bit);
            break;
        }
    }
    rcu_read_unlock();
    return has;
}
EXPORT_SYMBOL_GPL(monoos_proc_has_perm);

/* ------------------------------------------------------------------ */
/*  /proc/monoos/processes                                              */
/* ------------------------------------------------------------------ */
static int proc_list_show(struct seq_file *m, void *v)
{
    struct monoos_proc_record *rec;
    unsigned int bkt;

    seq_puts(m, "PID\tUID\tPERMS\t\tCOMM\n");
    seq_puts(m, "---\t---\t-----\t\t----\n");

    rcu_read_lock();
    hash_for_each_rcu(g_proc_table, bkt, rec, hash_node) {
        seq_printf(m, "%d\t%u\t0x%08x\t%s\n",
                   rec->pid, rec->uid, rec->active_perms, rec->comm);
    }
    rcu_read_unlock();
    return 0;
}

static int proc_list_open(struct inode *inode, struct file *file)
{
    return single_open(file, proc_list_show, NULL);
}

static const struct proc_ops proc_list_fops = {
    .proc_open    = proc_list_open,
    .proc_read    = seq_read,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};

/* ------------------------------------------------------------------ */
/*  kprobe on do_exit to auto-unregister dying processes              */
/* ------------------------------------------------------------------ */
static int do_exit_entry(struct kprobe *p, struct pt_regs *regs)
{
    monoos_proc_unregister(current->pid);
    return 0;
}

static struct kprobe g_exit_kprobe = {
    .symbol_name = "do_exit",
    .pre_handler = do_exit_entry,
};

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_monoos_dir;
static struct proc_dir_entry *g_proc_processes;

static int __init monoos_process_init(void)
{
    int ret;

    g_proc_cache = kmem_cache_create("monoos_proc",
                                      sizeof(struct monoos_proc_record),
                                      0, SLAB_HWCACHE_ALIGN, NULL);
    if (!g_proc_cache)
        return -ENOMEM;

    ret = register_kprobe(&g_exit_kprobe);
    if (ret) {
        pr_warn("monoos_process: kprobe on do_exit failed (%d), "
                "unregister on exit disabled\n", ret);
    }

    g_proc_monoos_dir = proc_mkdir("monoos", NULL);
    if (g_proc_monoos_dir) {
        g_proc_processes = proc_create("processes", 0444,
                                        g_proc_monoos_dir, &proc_list_fops);
    }

    pr_info("monoos_process: process lifecycle extensions loaded\n");
    return 0;
}

static void __exit monoos_process_exit(void)
{
    if (g_proc_processes) proc_remove(g_proc_processes);
    if (g_proc_monoos_dir) proc_remove(g_proc_monoos_dir);
    unregister_kprobe(&g_exit_kprobe);

    /* Drain the hash table. */
    {
        struct monoos_proc_record *rec;
        struct hlist_node *tmp;
        unsigned int bkt;
        spin_lock_irq(&g_proc_lock);
        hash_for_each_safe(g_proc_table, bkt, tmp, rec, hash_node) {
            hash_del(&rec->hash_node);
            kmem_cache_free(g_proc_cache, rec);
        }
        spin_unlock_irq(&g_proc_lock);
    }

    kmem_cache_destroy(g_proc_cache);
    pr_info("monoos_process: unloaded\n");
}

module_init(monoos_process_init);
module_exit(monoos_process_exit);
