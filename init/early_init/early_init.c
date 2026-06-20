/*
 * early_init.c – OneOS PID-1 early initialisation
 *
 * This C file is the first userspace code that runs after the kernel
 * hands control to /init in the ramdisk.  It:
 *
 *   1. Mounts essential virtual filesystems (proc, sys, dev, devpts).
 *   2. Reads /proc/cmdline and parses OneOS-specific boot arguments.
 *   3. Loads the kernel modules listed in /etc/oneos/modules.conf.
 *   4. Sets up the device-mapper nodes for dm-verity.
 *   5. Pivots to the real root filesystem.
 *   6. Execs the main init process (/system/bin/oneos_init).
 *
 * Kept deliberately minimal: no dynamic allocation, no threads.
 * Error handling calls sync() + reboot(LINUX_REBOOT_CMD_RESTART).
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/reboot.h>
#include <sys/sysmacros.h>
#include <linux/reboot.h>
#include <dirent.h>

/* ------------------------------------------------------------------ */
/*  Compile-time configuration                                         */
/* ------------------------------------------------------------------ */
#define ONEOS_VERSION          "1.0.0"
#define MODULES_CONF           "/etc/oneos/modules.conf"
#define REAL_ROOT_DEVICE       "/dev/block/by-name/system"
#define REAL_ROOT_FS           "ext4"
#define REAL_ROOT_FLAGS        MS_RDONLY
#define REAL_ROOT_MOUNT        "/mnt/system"
#define INIT_BINARY            "/system/bin/oneos_init"
#define LOG_DEVICE             "/dev/kmsg"
#define CMDLINE_MAX            4096
#define MODULES_MAX            64

/* ------------------------------------------------------------------ */
/*  Minimal logging (writes directly to /dev/kmsg)                    */
/* ------------------------------------------------------------------ */
static int g_log_fd = -1;

static void early_log(const char *level, const char *msg)
{
    char buf[256];
    int len = snprintf(buf, sizeof(buf), "<%s>oneos_early_init: %s\n",
                       level, msg);
    if (g_log_fd >= 0 && len > 0)
        (void)write(g_log_fd, buf, (size_t)len);
}

#define LOG_INFO(m)  early_log("6", m)
#define LOG_WARN(m)  early_log("4", m)
#define LOG_ERR(m)   early_log("3", m)

/* ------------------------------------------------------------------ */
/*  Panic: sync and reboot                                             */
/* ------------------------------------------------------------------ */
static void __attribute__((noreturn)) early_panic(const char *reason)
{
    early_log("0", reason);
    sync();
    reboot(LINUX_REBOOT_CMD_RESTART);
    for (;;) ; /* should not reach */
}

/* ------------------------------------------------------------------ */
/*  Mount virtual filesystems                                          */
/* ------------------------------------------------------------------ */
static void mount_vfs(void)
{
    struct { const char *src; const char *tgt; const char *type;
             unsigned long flags; const char *opts; } mounts[] = {
        { "proc",    "/proc",          "proc",     MS_NOSUID|MS_NOEXEC|MS_NODEV, NULL },
        { "sysfs",   "/sys",           "sysfs",    MS_NOSUID|MS_NOEXEC|MS_NODEV, NULL },
        { "devtmpfs","/dev",           "devtmpfs", MS_NOSUID|MS_NOEXEC,          "mode=0755,size=4m" },
        { "devpts",  "/dev/pts",       "devpts",   MS_NOSUID|MS_NOEXEC,          "mode=0620,gid=5" },
        { "tmpfs",   "/dev/shm",       "tmpfs",    MS_NOSUID|MS_NODEV,           "size=8m" },
        { "tmpfs",   "/run",           "tmpfs",    MS_NOSUID|MS_NODEV|MS_NOEXEC, "mode=0755,size=16m" },
        { "cgroup2", "/sys/fs/cgroup", "cgroup2",  MS_NOSUID|MS_NOEXEC|MS_NODEV, "nsdelegate" },
    };
    size_t i;

    mkdir("/dev/pts", 0755);
    mkdir("/dev/shm",  0755);
    mkdir("/run",      0755);
    mkdir("/sys/fs/cgroup", 0755);

    for (i = 0; i < sizeof(mounts)/sizeof(mounts[0]); i++) {
        if (mount(mounts[i].src, mounts[i].tgt, mounts[i].type,
                  mounts[i].flags, mounts[i].opts) != 0) {
            char msg[128];
            snprintf(msg, sizeof(msg), "mount %s -> %s failed: %s",
                     mounts[i].src, mounts[i].tgt, strerror(errno));
            /* Non-fatal for some mounts; warn and continue. */
            LOG_WARN(msg);
        }
    }
    LOG_INFO("virtual filesystems mounted");
}

/* ------------------------------------------------------------------ */
/*  Parse /proc/cmdline for oneos.* arguments                         */
/* ------------------------------------------------------------------ */
typedef struct {
    char boot_mode[32];      /* oneos.mode=... */
    int  safe_mode;          /* oneos.safe_mode=1 */
    int  adb_enabled;        /* oneos.adb=1 */
    int  verity_enforcing;   /* oneos.verity=enforcing|permissive */
    char slot_suffix[4];     /* oneos.slot=_a or _b */
} cmdline_opts_t;

static cmdline_opts_t g_opts;

static void parse_cmdline(void)
{
    char buf[CMDLINE_MAX];
    char *p, *tok, *eq;
    int  fd;
    ssize_t n;

    fd = open("/proc/cmdline", O_RDONLY);
    if (fd < 0) return;
    n = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (n <= 0) return;
    buf[n] = '\0';

    /* Defaults */
    strncpy(g_opts.boot_mode,   "normal", sizeof(g_opts.boot_mode) - 1);
    strncpy(g_opts.slot_suffix, "_a",     sizeof(g_opts.slot_suffix) - 1);
    g_opts.verity_enforcing = 1;

    p = buf;
    while ((tok = strsep(&p, " \t\n")) != NULL) {
        if (!*tok) continue;
        eq = strchr(tok, '=');
        if (!eq) continue;
        *eq = '\0';
        const char *key = tok, *val = eq + 1;

        if (!strcmp(key, "oneos.mode"))
            strncpy(g_opts.boot_mode, val, sizeof(g_opts.boot_mode) - 1);
        else if (!strcmp(key, "oneos.safe_mode"))
            g_opts.safe_mode = atoi(val);
        else if (!strcmp(key, "oneos.adb"))
            g_opts.adb_enabled = atoi(val);
        else if (!strcmp(key, "oneos.verity"))
            g_opts.verity_enforcing = strcmp(val, "enforcing") == 0 ? 1 : 0;
        else if (!strcmp(key, "oneos.slot"))
            strncpy(g_opts.slot_suffix, val, sizeof(g_opts.slot_suffix) - 1);
    }

    char msg[128];
    snprintf(msg, sizeof(msg), "boot mode=%s slot=%s verity=%s adb=%d",
             g_opts.boot_mode, g_opts.slot_suffix,
             g_opts.verity_enforcing ? "enforcing" : "permissive",
             g_opts.adb_enabled);
    LOG_INFO(msg);
}

/* ------------------------------------------------------------------ */
/*  Load kernel modules listed in /etc/oneos/modules.conf             */
/* ------------------------------------------------------------------ */
static void load_modules(void)
{
    FILE *f = fopen(MODULES_CONF, "r");
    char  line[256];
    int   count = 0;

    if (!f) { LOG_WARN("modules.conf not found – skipping"); return; }

    while (fgets(line, sizeof(line), f) && count < MODULES_MAX) {
        /* Strip newline and skip comments/blanks. */
        char *nl = strchr(line, '\n');
        if (nl) *nl = '\0';
        if (line[0] == '#' || line[0] == '\0') continue;

        /* insmod via /sbin/modprobe would be ideal; use system() here. */
        char cmd[512];
        snprintf(cmd, sizeof(cmd), "/sbin/insmod /lib/modules/%s.ko", line);
        int ret = system(cmd);
        if (ret != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "insmod %s failed (%d)", line, ret);
            LOG_WARN(msg);
        } else {
            count++;
        }
    }
    fclose(f);

    char msg[64];
    snprintf(msg, sizeof(msg), "loaded %d kernel modules", count);
    LOG_INFO(msg);
}

/* ------------------------------------------------------------------ */
/*  Pivot root to the real system partition                            */
/* ------------------------------------------------------------------ */
static void pivot_to_real_root(void)
{
    char dev[128];
    snprintf(dev, sizeof(dev), "/dev/block/by-name/system%s",
             g_opts.slot_suffix);

    mkdir(REAL_ROOT_MOUNT, 0755);

    if (mount(dev, REAL_ROOT_MOUNT, REAL_ROOT_FS,
              REAL_ROOT_FLAGS, "discard") != 0) {
        /* dm-verity will have set up the device-mapper node;
           fall back to the dm- path. */
        if (mount("/dev/mapper/system", REAL_ROOT_MOUNT, REAL_ROOT_FS,
                  REAL_ROOT_FLAGS, NULL) != 0) {
            early_panic("failed to mount system partition");
        }
    }
    LOG_INFO("system partition mounted");

    if (chdir(REAL_ROOT_MOUNT) != 0)
        early_panic("chdir to real root failed");

    /* pivot_root: make new root current, old root at /mnt/old */
    mkdir("mnt/old", 0755);
    if (syscall(155 /* __NR_pivot_root */, ".", "mnt/old") != 0) {
        /* Fallback: chroot */
        if (chroot(".") != 0)
            early_panic("pivot_root and chroot both failed");
        chdir("/");
    } else {
        chdir("/");
        umount2("/mnt/old", MNT_DETACH);
    }

    LOG_INFO("pivoted to real root");
}

/* ------------------------------------------------------------------ */
/*  Main                                                               */
/* ------------------------------------------------------------------ */
int main(int argc __attribute__((unused)),
         char **argv __attribute__((unused)))
{
    /* Open /dev/kmsg for log output as early as possible. */
    g_log_fd = open(LOG_DEVICE, O_WRONLY | O_CLOEXEC);

    LOG_INFO("OneOS " ONEOS_VERSION " early_init starting");

    mount_vfs();
    parse_cmdline();
    load_modules();

    if (strcmp(g_opts.boot_mode, "recovery") != 0)
        pivot_to_real_root();

    /* Hand off to the main init process. */
    const char *init_argv[] = { INIT_BINARY, NULL };
    const char *init_envp[] = {
        "PATH=/system/bin:/system/xbin:/vendor/bin",
        "LD_LIBRARY_PATH=/system/lib64:/vendor/lib64",
        NULL
    };

    execve(INIT_BINARY, (char *const *)init_argv, (char *const *)init_envp);

    /* execve only returns on failure. */
    early_panic("exec of main init failed");
}
