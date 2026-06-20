# Makefile for oneos_mm out-of-tree kernel module
obj-m += oneos_mm.o
oneos_mm-objs := mm.o

KDIR ?= /lib/modules/$(shell uname -r)/build

all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
