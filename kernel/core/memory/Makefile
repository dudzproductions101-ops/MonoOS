# Makefile for monoos_mm out-of-tree kernel module
obj-m += monoos_mm.o
monoos_mm-objs := mm.o

KDIR ?= /lib/modules/$(shell uname -r)/build

all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
