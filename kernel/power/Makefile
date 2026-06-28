obj-m += monoos_power.o
ccflags-y += -I$(src)/../../include
monoos_power-objs := power_manager.o
KDIR ?= /lib/modules/$(shell uname -r)/build
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
