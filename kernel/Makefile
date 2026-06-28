# MonoOS kernel modules – top-level Makefile
#
# Build order matters for cross-module EXPORT_SYMBOL_GPL resolution:
#   1. monoos_process  (exports monoos_proc_{register,has_perm,...})
#   2. monoos_mm       (standalone)
#   3. monoos_sched    (standalone, imports nothing else from MonoOS)
#   4. monoos_power    (standalone)
#   5. monoos_vfs      (depends on monoos_process for monoos_proc_has_perm)
#   6. monoos_net      (standalone)
#   7. monoos_lsm      (depends on monoos_process)
#   8. monoos_syscalls (depends on monoos_process, kprobes for kallsyms)
#
# Usage:
#   make -C /lib/modules/$(uname -r)/build M=$(pwd) modules
#   make -C /lib/modules/$(uname -r)/build M=$(pwd) modules_install
#   make -C /lib/modules/$(uname -r)/build M=$(pwd) clean

KDIR ?= /lib/modules/$(shell uname -r)/build

# Shared include path used by every submodule.
ccflags-y += -I$(src)/include

# ─── Core ────────────────────────────────────────────────────────────
obj-m += monoos_process.o
monoos_process-objs      := core/process/process.o

obj-m += monoos_mm.o
monoos_mm-objs           := core/memory/mm.o

obj-m += monoos_sched.o
monoos_sched-objs        := core/scheduler/sched.o

obj-m += monoos_syscalls.o
monoos_syscalls-objs     := core/syscalls/syscalls.o

# ─── Filesystem ──────────────────────────────────────────────────────
obj-m += monoos_vfs.o
monoos_vfs-objs          := filesystem/vfs_overlay.o

# ─── Networking ──────────────────────────────────────────────────────
obj-m += monoos_net.o
monoos_net-objs          := networking/netfilter_privacy.o

# ─── Power ───────────────────────────────────────────────────────────
obj-m += monoos_power.o
monoos_power-objs        := power/power_manager.o

# ─── Security ────────────────────────────────────────────────────────
obj-m += monoos_lsm.o
monoos_lsm-objs          := security/lsm_monoos.o

# ─── Build targets ───────────────────────────────────────────────────
all:
	$(MAKE) -C $(KDIR) M=$(shell pwd) modules

modules_install:
	$(MAKE) -C $(KDIR) M=$(shell pwd) modules_install

clean:
	$(MAKE) -C $(KDIR) M=$(shell pwd) clean
