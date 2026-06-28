obj-m += monoos_process.o
ccflags-y += -I$(src)/../../include
monoos_process-objs := process.o
KDIR ?= /lib/modules/$(shell uname -r)/build
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
