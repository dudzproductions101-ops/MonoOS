obj-m += monoos_syscalls.o
ccflags-y += -I$(src)/../../include
# kprobe-based kallsyms_lookup_name workaround requires CONFIG_KPROBES=y.
# The module will refuse to load if kprobes are not available.
monoos_syscalls-objs := syscalls.o
KDIR ?= /lib/modules/$(shell uname -r)/build
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
