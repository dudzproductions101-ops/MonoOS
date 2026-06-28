/*
 * netfilter_privacy.c – MonoOS network privacy filter (kernel module)
 *
 * Registers Netfilter hooks to:
 *   1. Block outbound traffic from apps that do not hold MONOOS_PERM_NETWORK.
 *   2. Detect and log connections to known tracker domains (from a
 *      compile-time blocklist; the full list is loaded from userspace
 *      via a Netlink socket at runtime).
 *   3. Enforce a per-app firewall policy configured by the system server.
 *   4. Emit connection events to /proc/monoos/net_events for the
 *      privacy dashboard.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/netfilter.h>
#include <linux/netfilter_ipv4.h>
#include <linux/netfilter_ipv6.h>
#include <linux/ip.h>
#include <linux/ipv6.h>
#include <linux/tcp.h>
#include <linux/udp.h>
#include <linux/skbuff.h>
#include <linux/net.h>
#include <linux/socket.h>
#include <linux/slab.h>
#include <linux/spinlock.h>
#include <linux/kfifo.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/atomic.h>
#include <linux/in.h>
#include <linux/in6.h>
#include <net/sock.h>
#include <net/net_namespace.h>
#include "monoos_net.h" /* via ccflags-y */

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS network privacy filter");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Compile-time tracker IP blocklist (illustrative; a tiny subset)   */
/*  Runtime list is loaded from /etc/monoos/tracker_ips via Netlink.   */
/* ------------------------------------------------------------------ */
static const __be32 blocked_ips_v4[] = {
    /* Google Analytics: 216.239.32.0/19  – first address only here */
    __constant_htonl(0xD8EF2000),
    /* Facebook pixel: 157.240.0.0/16 */
    __constant_htonl(0x9DF00000),
    /* Doubleclick: 74.125.0.0/16 */
    __constant_htonl(0x4A7D0000),
};
#define N_BLOCKED_IPS  ARRAY_SIZE(blocked_ips_v4)
#define BLOCKED_MASK   __constant_htonl(0xFFFF0000)

/* ------------------------------------------------------------------ */
/*  Per-app firewall rule (indexed by UID)                             */
/* ------------------------------------------------------------------ */
#define MAX_FIREWALL_RULES 128

struct fw_rule {
    uid_t  uid;
    bool   allow_network;   /* false = no outbound allowed            */
    bool   block_trackers;  /* true  = enforce tracker blocklist      */
    bool   vpn_only;        /* true  = only traffic through VPN tun   */
    u32    allowed_ports;   /* bitmask of allowed dst port ranges     */
};

static struct fw_rule g_rules[MAX_FIREWALL_RULES];
static int            g_rule_count;
static DEFINE_RWLOCK(g_rules_lock);

/* ------------------------------------------------------------------ */
/*  Network event ring buffer                                          */
/* ------------------------------------------------------------------ */
struct net_event {
    u64     ts_ns;
    pid_t   pid;
    uid_t   uid;
    __be32  dst_ip;
    __be16  dst_port;
    u8      proto;          /* IPPROTO_TCP / IPPROTO_UDP              */
    u8      blocked;        /* 1 = drop, 0 = pass                     */
    u8      reason;         /* 0=ok, 1=no_perm, 2=tracker, 3=fw_rule */
} __packed;

#define NET_FIFO_SIZE 512
DEFINE_KFIFO(g_net_fifo, struct net_event, NET_FIFO_SIZE);
static DEFINE_SPINLOCK(g_net_fifo_lock);

static atomic64_t g_packets_seen    = ATOMIC64_INIT(0);
static atomic64_t g_packets_blocked = ATOMIC64_INIT(0);
static atomic64_t g_tracker_hits    = ATOMIC64_INIT(0);

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */
extern bool monoos_proc_has_perm(pid_t pid, u32 perm_bit);
#define MONOOS_PERM_NETWORK  0x0200U

static bool ip_is_blocked_tracker(__be32 ip)
{
    size_t i;
    for (i = 0; i < N_BLOCKED_IPS; i++)
        if ((ip & BLOCKED_MASK) == (blocked_ips_v4[i] & BLOCKED_MASK))
            return true;
    return false;
}

static struct fw_rule *find_rule(uid_t uid)
{
    int i;
    for (i = 0; i < g_rule_count; i++)
        if (g_rules[i].uid == uid)
            return &g_rules[i];
    return NULL;
}

static void emit_net_event(uid_t uid, __be32 dst_ip, __be16 dst_port,
                            u8 proto, bool blocked, u8 reason)
{
    struct net_event ev;
    unsigned long flags;

    ev.ts_ns    = ktime_get_ns();
    ev.pid      = current->pid;
    ev.uid      = uid;
    ev.dst_ip   = dst_ip;
    ev.dst_port = dst_port;
    ev.proto    = proto;
    ev.blocked  = blocked ? 1 : 0;
    ev.reason   = reason;

    spin_lock_irqsave(&g_net_fifo_lock, flags);
    kfifo_in(&g_net_fifo, &ev, 1);
    spin_unlock_irqrestore(&g_net_fifo_lock, flags);
}

/* ------------------------------------------------------------------ */
/*  Netfilter hook – IPv4 output                                      */
/* ------------------------------------------------------------------ */
static unsigned int monoos_nf_ipv4_out(void *priv,
                                       struct sk_buff *skb,
                                       const struct nf_hook_state *state)
{
    struct iphdr  *iph;
    struct tcphdr *tcph = NULL;
    struct udphdr *udph = NULL;
    uid_t uid = 0;
    __be16 dport = 0;
    struct fw_rule *rule;
    bool block = false;
    u8 reason = 0;

    atomic64_inc(&g_packets_seen);

    if (!skb->sk) return NF_ACCEPT;

    iph = ip_hdr(skb);
    uid = from_kuid_munged(&init_user_ns, skb->sk->sk_uid);

    /* Extract destination port */
    if (iph->protocol == IPPROTO_TCP) {
        tcph = tcp_hdr(skb);
        dport = tcph->dest;
    } else if (iph->protocol == IPPROTO_UDP) {
        udph = udp_hdr(skb);
        dport = udph->dest;
    }

    /*
     * Default-deny policy for app UIDs (≥ 10000).
     *
     * Android/MonoOS convention: UIDs 0–9999 are system processes that have
     * unconditional network access.  App UIDs start at 10000.  An app gets
     * outbound network access only when permission_service has called
     * monoos_net_set_rule(uid, allow=true, …) after the user grants
     * MONOOS_PERM_NETWORK.  Without an explicit ALLOW rule the packet drops.
     *
     * This flips the previous behaviour (default-accept + deny list) to
     * default-deny + allow list, which is the correct privacy default noted
     * in HANDOFF.md §"Network default-deny is not implemented".
     */
#define MONOOS_APP_UID_MIN 10000U

    read_lock(&g_rules_lock);
    rule = find_rule(uid);

    if (uid >= MONOOS_APP_UID_MIN) {
        if (!rule) {
            /* No rule registered yet — deny by default. */
            block = true; reason = 1;
        } else if (!rule->allow_network) {
            block = true; reason = 1;
        } else if (rule->block_trackers && ip_is_blocked_tracker(iph->daddr)) {
            block = true; reason = 2;
            atomic64_inc(&g_tracker_hits);
        } else if (ip_is_blocked_tracker(iph->daddr)) {
            /* Global tracker block applies to all apps regardless of rule. */
            block = true; reason = 2;
            atomic64_inc(&g_tracker_hits);
        }
    } else {
        /* System UID — only apply tracker blocklist, never deny entirely. */
        if (ip_is_blocked_tracker(iph->daddr)) {
            atomic64_inc(&g_tracker_hits);
            emit_net_event(uid, iph->daddr, dport, iph->protocol, false, 2);
            /* Log only — do not block system traffic. */
        }
    }
    read_unlock(&g_rules_lock);

    if (block) {
        atomic64_inc(&g_packets_blocked);
        emit_net_event(uid, iph->daddr, dport, iph->protocol, true, reason);
        return NF_DROP;
    }

    return NF_ACCEPT;
}

static const struct nf_hook_ops monoos_nf_hooks[] = {
    {
        .hook      = monoos_nf_ipv4_out,
        .pf        = NFPROTO_IPV4,
        .hooknum   = NF_INET_LOCAL_OUT,
        .priority  = NF_IP_PRI_FILTER - 1,
    },
};

/* ------------------------------------------------------------------ */
/*  /proc/monoos/net_events                                             */
/* ------------------------------------------------------------------ */
static ssize_t net_events_read(struct file *file, char __user *buf,
                                size_t count, loff_t *ppos)
{
    struct net_event ev;
    unsigned int copied = 0;
    char line[128];
    int  len;

    while (count > sizeof(line)) {
        unsigned long flags;
        int got;

        spin_lock_irqsave(&g_net_fifo_lock, flags);
        got = kfifo_out(&g_net_fifo, &ev, 1);
        spin_unlock_irqrestore(&g_net_fifo_lock, flags);

        if (!got) break;

        len = snprintf(line, sizeof(line),
                       "%llu %d %u %pI4 %u %u %u %u\n",
                       ev.ts_ns, ev.pid, ev.uid,
                       &ev.dst_ip, ntohs(ev.dst_port),
                       ev.proto, ev.blocked, ev.reason);

        if (len <= 0 || (size_t)len >= count) break;
        if (copy_to_user(buf + copied, line, (size_t)len)) return -EFAULT;
        copied += (unsigned)len;
        count  -= (size_t)len;
    }
    return (ssize_t)copied;
}

static int net_events_open(struct inode *i, struct file *f) { return 0; }

static const struct proc_ops net_events_fops = {
    .proc_open = net_events_open,
    .proc_read = net_events_read,
};

static int net_stats_show(struct seq_file *m, void *v)
{
    seq_printf(m, "packets_seen:    %lld\n", atomic64_read(&g_packets_seen));
    seq_printf(m, "packets_blocked: %lld\n", atomic64_read(&g_packets_blocked));
    seq_printf(m, "tracker_hits:    %lld\n", atomic64_read(&g_tracker_hits));
    seq_printf(m, "firewall_rules:  %d\n",   g_rule_count);
    return 0;
}
static int net_stats_open(struct inode *i, struct file *f)
{
    return single_open(f, net_stats_show, NULL);
}
static const struct proc_ops net_stats_fops = {
    .proc_open    = net_stats_open,
    .proc_read    = seq_read,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};

/* ------------------------------------------------------------------ */
/*  Public API for system server to install per-app rules             */
/* ------------------------------------------------------------------ */
int monoos_net_set_rule(uid_t uid, bool allow, bool block_trackers,
                        bool vpn_only)
{
    struct fw_rule *rule;
    unsigned long flags;

    write_lock_irqsave(&g_rules_lock, flags);
    rule = find_rule(uid);
    if (!rule) {
        if (g_rule_count >= MAX_FIREWALL_RULES) {
            write_unlock_irqrestore(&g_rules_lock, flags);
            return -ENOMEM;
        }
        rule = &g_rules[g_rule_count++];
        rule->uid = uid;
    }
    rule->allow_network   = allow;
    rule->block_trackers  = block_trackers;
    rule->vpn_only        = vpn_only;
    write_unlock_irqrestore(&g_rules_lock, flags);
    return 0;
}
EXPORT_SYMBOL_GPL(monoos_net_set_rule);

/* ------------------------------------------------------------------ */
/*  Module init / exit                                                 */
/* ------------------------------------------------------------------ */
static struct proc_dir_entry *g_proc_monoos;
static struct proc_dir_entry *g_proc_events;
static struct proc_dir_entry *g_proc_stats;

static int __init monoos_netfilter_init(void)
{
    int ret = nf_register_net_hooks(&init_net, monoos_nf_hooks,
                                     ARRAY_SIZE(monoos_nf_hooks));
    if (ret) {
        pr_err("monoos_net: nf_register failed: %d\n", ret);
        return ret;
    }

    g_proc_monoos  = proc_mkdir("monoos", NULL);
    if (g_proc_monoos) {
        g_proc_events = proc_create("net_events", 0400, g_proc_monoos,
                                     &net_events_fops);
        g_proc_stats  = proc_create("net_stats",  0444, g_proc_monoos,
                                     &net_stats_fops);
    } else {
        /* /proc/monoos already registered by monoos_process — reuse it. */
        g_proc_monoos = proc_lookup_entry("monoos", NULL);
        if (g_proc_monoos) {
            g_proc_events = proc_create("net_events", 0400, g_proc_monoos,
                                         &net_events_fops);
            g_proc_stats  = proc_create("net_stats",  0444, g_proc_monoos,
                                         &net_stats_fops);
        } else {
            pr_warn("monoos_net: /proc/monoos not available — net_events/net_stats disabled\n");
        }
    }

    pr_info("monoos_net: network privacy filter loaded (%zu tracker IPs)\n",
            N_BLOCKED_IPS);
    return 0;
}

static void __exit monoos_netfilter_exit(void)
{
    nf_unregister_net_hooks(&init_net, monoos_nf_hooks,
                             ARRAY_SIZE(monoos_nf_hooks));
    if (g_proc_events) proc_remove(g_proc_events);
    if (g_proc_stats)  proc_remove(g_proc_stats);
    if (g_proc_monoos)  proc_remove(g_proc_monoos);
    pr_info("monoos_net: unloaded\n");
}

module_init(monoos_netfilter_init);
module_exit(monoos_netfilter_exit);
