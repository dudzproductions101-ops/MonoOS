obj-m += monoos_vfs.o
ccflags-y += -I$(src)/../../include
monoos_vfs-objs := vfs_overlay.o
KDIR ?= /lib/modules/$(shell uname -r)/build
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
