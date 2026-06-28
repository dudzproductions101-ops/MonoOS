obj-m += monoos_lsm.o
ccflags-y += -I$(src)/../../include
monoos_lsm-objs := lsm_monoos.o
KDIR ?= /lib/modules/$(shell uname -r)/build
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
