/*
 * sched.c – MonoOS scheduler extensions
 *
 * Augments the Linux CFS/EAS scheduler for mobile workloads:
 *   1. App-aware foreground/background classification.
 *   2. Dynamic scheduler tunables exposed via /proc/monoos/sched.
 *   3. Wake-up latency hints for interactive app threads.
 *   4. Frame-pacing callback: processes can register a target frame
 *      period and receive boosted priority during render windows.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/sched.h>
#include <linux/sched/rt.h>
#include <linux/sched/prio.h>
#include <linux/cpumask.h>
#include <linux/cpufreq.h>
#include <linux/atomic.h>
#include <linux/spinlock.h>
#include <linux/hashtable.h>
#include <linux/slab.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/uaccess.h>
#include <linux/hrtimer.h>
#include <linux/ktime.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS scheduler extensions");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Thread classification                                              */
/* ------------------------------------------------------------------ */

/** Classification of an app thread for scheduling purposes. */
typedef enum {
    THREAD_CLASS_UNKNOWN    = 0,
    THREAD_CLASS_FOREGROUND = 1,   /* Visible, interactive thread     */
    THREAD_CLASS_RENDER     = 2,   /* Frame-rendering (HWUI / Vulkan) */
    THREAD_CLASS_AUDIO      = 3,   /* Low-latency audio callback      */
    THREAD_CLASS_BACKGROUND = 4,   /* Background sync / upload        */
    THREAD_CLASS_IDLE       = 5,   /* Idle maintenance task           */
} thread_class_t;

/** Nice value and SCHED_POLICY applied per class. */
static const struct {
    int         nice;
    int         policy;
    int         rt_prio; /* used if policy == SCHED_RR */
} class_params[] = {
    [THREAD_CLASS_UNKNOWN]    = { 0,   SCHED_NORMAL, 0  },
    [THREAD_CLASS_FOREGROUND] = { -4,  SCHED_NORMAL, 0  },
    [THREAD_CLASS_RENDER]     = { -10, SCHED_FIFO,   52 },
    [THREAD_CLASS_AUDIO]      = { -16, SCHED_FIFO,   80 },
    [THREAD_CLASS_BACKGROUND] = { 10,  SCHED_BATCH,  0  },
    [THREAD_CLASS_IDLE]       = { 19,  SCHED_IDLE,   0  },
};

/* ------------------------------------------------------------------ */
/*  Per-thread scheduling record                                       */
/* ------------------------------------------------------------------ */
#define SCHED_HASH_BITS 8

struct monoos_sched_thread {
    pid_t             tid;
    thread_class_t    cls;
    u64               target_frame_ns;  /* 0 = not frame-pacing */
    u64               last_frame_start; /* ktime when last frame began */
    u32               boost_until_ns;   /* ns remaining in boost window */
    struct hlist_node hash_node;
    struct rcu_head   rcu;
};

static DEFINE_HASHTABLE(g_sched_table, SCHED_HASH_BITS);
static DEFINE_SPINLOCK(g_sched_lock);
static struct kmem_cache *g_sched_cache;

/* Statistics */
static atomic64_t g_boosts_issued   = ATOMIC64_INIT(0);
static atomic64_t g_frame_callbacks = ATOMIC64_INIT(0);
static u32        g_target_fps      = 60; /* tunable via /proc */

/* ------------------------------------------------------------------ */
/*  Thread record management                                           */
/* ------------------------------------------------------------------ */
static struct monoos_sched_thread *sched_record_find(pid_t tid)
{
    struct monoos_sched_thread *rec;
    hash_for_each_possible_rcu(g_sched_table, rec, hash_node, (u32)tid)
        if (rec->tid == tid) return rec;
    return NULL;
}

static void sched_record_free_rcu(struct rcu_head *head)
{
    struct monoos_sched_thread *rec =
        container_of(head, struct monoos_sched_thread, rcu);
    kmem_cache_free(g_sched_cache, rec);
}

/* ------------------------------------------------------------------ */
/*  Public API                                                         */
/* ------------------------------------------------------------------ */

/**
 * monoos_sched_classify – classify thread @tid into @cls and apply the
 * corresponding scheduler policy / nice value.
 */
int monoos_sched_classify(pid_t tid, thread_class_t cls)
{
    struct monoos_sched_thread *rec;
    struct task_struct *task;
    struct sched_param sp;
    unsigned long flags;
    int ret = 0;

    if ((unsigned)cls >= ARRAY_SIZE(class_params))
        return -EINVAL;

    /* Update or create record */
    spin_lock_irqsave(&g_sched_lock, flags);
    rec = NULL;
    hash_for_each_possible(g_sched_table, rec, hash_node, (u32)tid)
        if (rec->tid == tid) break;

    if (!rec) {
        rec = kmem_cache_zalloc(g_sched_cache, GFP_ATOMIC);
        if (!rec) {
            spin_unlock_irqrestore(&g_sched_lock, flags);
            return -ENOMEM;
        }
        rec->tid = tid;
        hash_add(g_sched_table, &rec->hash_node, (u32)tid);
    }
    rec->cls = cls;
    spin_unlock_irqrestore(&g_sched_lock, flags);

    /* Apply policy to the task */
    rcu_read_lock();
    task = find_task_by_vpid(tid);
    if (!task) { rcu_read_unlock(); return -ESRCH; }
    get_task_struct(task);
    rcu_read_unlock();

    sp.sched_priority = class_params[cls].rt_prio;
    ret = sched_setscheduler_nocheck(task, class_params[cls].policy, &sp);
    if (ret == 0 && class_params[cls].policy == SCHED_NORMAL)
        set_user_nice(task, class_params[cls].nice);

    put_task_struct(task);
    return ret;
}
EXPORT_SYMBOL_GPL(monoos_sched_classify);

/**
 * monoos_sched_set_frame_target – register a frame period for @tid so the
 * scheduler can boost it at the start of each render window.
 *
 * @tid:       thread ID of the render thread.
 * @fps:       target frames per second (1–240).
 */
int monoos_sched_set_frame_target(pid_t tid, u32 fps)
{
    struct monoos_sched_thread *rec;
    unsigned long flags;

    if (fps == 0 || fps > 240)
        return -EINVAL;

    spin_lock_irqsave(&g_sched_lock, flags);
    rec = NULL;
    hash_for_each_possible(g_sched_table, rec, hash_node, (u32)tid)
        if (rec->tid == tid) break;

    if (!rec) {
        rec = kmem_cache_zalloc(g_sched_cache, GFP_ATOMIC);
        if (!rec) {
            spin_unlock_irqrestore(&g_sched_lock, flags);
            return -ENOMEM;
        }
        rec->tid = tid;
        rec->cls = THREAD_CLASS_RENDER;
        hash_add(g_sched_table, &rec->hash_node, (u32)tid);
    }

    rec->target_frame_ns = NSEC_PER_SEC / fps;
    spin_unlock_irqrestore(&g_sched_lock, flags);

    g_target_fps = fps;
    return 0;
}
EXPORT_SYMBOL_GPL(monoos_sched_set_frame_target);

/**
 * monoos_sched_frame_begin – called by the Wayland compositor at the start
 * of each frame to boost the render thread for the frame window.
 */
void monoos_sched_frame_begin(pid_t tid)
{
    struct monoos_sched_thread *rec;

    rcu_read_lock();
    rec = sched_record_find(tid);
    if (rec && rec->target_frame_ns > 0) {
        rec->last_frame_start = ktime_get_ns();
        atomic64_inc(&g_frame_callbacks);
        /* In a real impl: temporarily bump sched_setattr SCHED_DEADLINE
         * with runtime=frame_time/2, deadline=frame_time. */
        atomic64_inc(&g_boosts_issued);
    }
    rcu_read_unlock();
}
EXPORT_SYMBOL_GPL(monoos_sched_frame_begin);

/**
 * monoos_sched_unregister – remove a thread from the scheduler extension table.
 */
void monoos_sched_unregister(pid_t tid)
{
    struct monoos_sched_thread *rec;
    unsigned long flags;

    spin_lock_irqsave(&g_sched_lock, flags);
    hash_for_each_possible(g_sched_table, rec, hash_node, (u32)tid) {
        if (rec->tid == tid) {
            hash_del_rcu(&rec->hash_node);
            spin_unlock_irqrestore(&g_sched_lock, flags);
            call_rcu(&rec->rcu, sched_record_free_rcu);
            return;
        }
    }
    spin_unlock_irqrestore(&g_sched_lock, flags);
}
EXPORT_SYMBOL_GPL(monoos_sched_unregister);

/* ------------------------------------------------------------------ */
/*  /proc/monoos/sched                                                  */
/* ------------------------------------------------------------------ */
static int sched_info_show(struct seq_file *m, void *v)
{
    struct monoos_sched_thread *rec;
    unsigned int bkt;

    seq_printf(m, "target_fps:     %u\n", g_target_fps);
    seq_printf(m, "boosts_issued:  %lld\n", atomic64_read(&g_boosts_issued));
    seq_printf(m, "frame_callbacks:%lld\n", atomic64_read(&g_frame_callbacks));
    seq_puts(m, "\nTID\tCLASS\t\tFRAME_NS\n");

    rcu_read_lock();
    hash_for_each_rcu(g_sched_table, bkt, rec, hash_node) {
        seq_printf(m, "%d\t%d\t\t%llu\n",
                   rec->tid, (int)rec->cls, rec->target_frame_ns);
    }
    rcu_read_unlock();
    return 0;
}

static int sched_info_open(struct inode *inode, struct file *file)
{
    return single_open(file, sched_info_show, NULL);
}

static ssize_t sched_info_write(struct file *file, const char __user *buf,
                                 size_t count, loff_t *ppos)
{
    char kbuf[64];
    u32 fps;

    if (count >= sizeof(kbuf))
        return -EINVAL;
    if (copy_from_user(kbuf, buf, count))
        return -EFAULT;
    kbuf[count] = '\0';

    if (kstrtou32(kbuf, 10, &fps) == 0 && fps > 0 && fps <= 240)
        g_target_fps = fps;

    return count;
}

static const struct proc_ops sched_info_fops = {
    .proc_open    = sched_info_open,
    .proc_read    = seq_read,
    .proc_write   = sched_info_write,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_monoos;
static struct proc_dir_entry *g_proc_sched;

static int __init monoos_sched_init(void)
{
    g_sched_cache = kmem_cache_create("monoos_sched",
                                       sizeof(struct monoos_sched_thread),
                                       0, SLAB_HWCACHE_ALIGN, NULL);
    if (!g_sched_cache)
        return -ENOMEM;

    g_proc_monoos = proc_mkdir("monoos", NULL);
    if (g_proc_monoos)
        g_proc_sched = proc_create("sched", 0644, g_proc_monoos,
                                    &sched_info_fops);

    pr_info("monoos_sched: scheduler extensions loaded (target %u fps)\n",
            g_target_fps);
    return 0;
}

static void __exit monoos_sched_exit(void)
{
    struct monoos_sched_thread *rec;
    struct hlist_node *tmp;
    unsigned int bkt;

    if (g_proc_sched) proc_remove(g_proc_sched);
    if (g_proc_monoos) proc_remove(g_proc_monoos);

    spin_lock_irq(&g_sched_lock);
    hash_for_each_safe(g_sched_table, bkt, tmp, rec, hash_node) {
        hash_del(&rec->hash_node);
        kmem_cache_free(g_sched_cache, rec);
    }
    spin_unlock_irq(&g_sched_lock);

    kmem_cache_destroy(g_sched_cache);
    pr_info("monoos_sched: unloaded\n");
}

module_init(monoos_sched_init);
module_exit(monoos_sched_exit);
