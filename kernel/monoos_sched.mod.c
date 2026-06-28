#include <linux/module.h>
#define INCLUDE_VERMAGIC
#include <linux/build-salt.h>
#include <linux/elfnote-lto.h>
#include <linux/export-internal.h>
#include <linux/vermagic.h>
#include <linux/compiler.h>

#ifdef CONFIG_UNWINDER_ORC
#include <asm/orc_header.h>
ORC_HEADER;
#endif

BUILD_SALT;
BUILD_LTO_INFO;

MODULE_INFO(vermagic, VERMAGIC_STRING);
MODULE_INFO(name, KBUILD_MODNAME);

__visible struct module __this_module
__section(".gnu.linkonce.this_module") = {
	.name = KBUILD_MODNAME,
	.init = init_module,
#ifdef CONFIG_MODULE_UNLOAD
	.exit = cleanup_module,
#endif
	.arch = MODULE_ARCH_INIT,
};

#ifdef CONFIG_MITIGATION_RETPOLINE
MODULE_INFO(retpoline, "Y");
#endif

KSYMTAB_FUNC(monoos_sched_classify, "_gpl", "");
KSYMTAB_FUNC(monoos_sched_set_frame_target, "_gpl", "");
KSYMTAB_FUNC(monoos_sched_frame_begin, "_gpl", "");
KSYMTAB_FUNC(monoos_sched_unregister, "_gpl", "");

SYMBOL_CRC(monoos_sched_classify, 0xa0371cbc, "_gpl");
SYMBOL_CRC(monoos_sched_set_frame_target, 0x88231945, "_gpl");
SYMBOL_CRC(monoos_sched_frame_begin, 0xa716019a, "_gpl");
SYMBOL_CRC(monoos_sched_unregister, 0x8ddbca3c, "_gpl");

static const char ____versions[]
__used __section("__versions") =
	"\x1c\x00\x00\x00\x48\x9f\xdb\x88"
	"__check_object_size\0"
	"\x18\x00\x00\x00\xc2\x9c\xc4\x13"
	"_copy_from_user\0"
	"\x18\x00\x00\x00\x14\x27\x52\x8d"
	"__rcu_read_lock\0"
	"\x14\x00\x00\x00\xfd\x16\x99\xce"
	"proc_create\0"
	"\x1c\x00\x00\x00\x07\x64\xa9\x7e"
	"sched_set_normal\0\0\0\0"
	"\x18\x00\x00\x00\x38\x9a\x02\xf7"
	"get_pid_task\0\0\0\0"
	"\x1c\x00\x00\x00\x7d\xf9\xc7\xc8"
	"__put_task_struct\0\0\0"
	"\x14\x00\x00\x00\xea\x41\x6c\x3b"
	"kstrtouint\0\0"
	"\x14\x00\x00\x00\xed\x38\xe1\x0a"
	"seq_lseek\0\0\0"
	"\x18\x00\x00\x00\xd8\x28\xdf\x72"
	"find_get_pid\0\0\0\0"
	"\x1c\x00\x00\x00\xcb\x60\x33\x14"
	"kmem_cache_create\0\0\0"
	"\x20\x00\x00\x00\x0b\x05\xdb\x34"
	"_raw_spin_lock_irqsave\0\0"
	"\x1c\x00\x00\x00\x9f\x5b\xf5\x04"
	"sched_set_fifo_low\0\0"
	"\x14\x00\x00\x00\xbb\x6d\xfb\xbd"
	"__fentry__\0\0"
	"\x20\x00\x00\x00\xa2\x54\x91\x9c"
	"sched_setattr_nocheck\0\0\0"
	"\x10\x00\x00\x00\x7e\x3a\x2c\x12"
	"_printk\0"
	"\x1c\x00\x00\x00\x7b\xcc\x27\x84"
	"_raw_spin_lock_irq\0\0"
	"\x1c\x00\x00\x00\xcb\xf6\xfd\xf0"
	"__stack_chk_fail\0\0\0\0"
	"\x20\x00\x00\x00\x5f\x69\x96\x02"
	"refcount_warn_saturate\0\0"
	"\x1c\x00\x00\x00\x44\x6a\x71\x11"
	"kmem_cache_alloc\0\0\0\0"
	"\x28\x00\x00\x00\xb3\x1c\xa2\x87"
	"__ubsan_handle_out_of_bounds\0\0\0\0"
	"\x1c\x00\x00\x00\x0f\x81\x69\x24"
	"__rcu_read_unlock\0\0\0"
	"\x18\x00\x00\x00\x57\x0f\x44\x4b"
	"kmem_cache_free\0"
	"\x20\x00\x00\x00\x53\x0f\x75\x4b"
	"_raw_spin_unlock_irq\0\0\0\0"
	"\x24\x00\x00\x00\x70\xce\x5c\xd3"
	"_raw_spin_unlock_irqrestore\0"
	"\x14\x00\x00\x00\x96\x01\xb0\x91"
	"proc_mkdir\0\0"
	"\x1c\x00\x00\x00\xca\x39\x82\x5b"
	"__x86_return_thunk\0\0"
	"\x14\x00\x00\x00\x73\x10\x33\xfe"
	"proc_remove\0"
	"\x14\x00\x00\x00\x32\x5b\xc6\x7c"
	"seq_read\0\0\0\0"
	"\x14\x00\x00\x00\x67\x6a\xaa\x28"
	"call_rcu\0\0\0\0"
	"\x14\x00\x00\x00\x65\x93\x3f\xb4"
	"ktime_get\0\0\0"
	"\x14\x00\x00\x00\x10\xd3\x2e\x16"
	"seq_printf\0\0"
	"\x14\x00\x00\x00\x26\x60\x86\x30"
	"seq_puts\0\0\0\0"
	"\x18\x00\x00\x00\x9f\xbe\xbe\xd3"
	"single_release\0\0"
	"\x18\x00\x00\x00\xda\x47\x88\xf2"
	"sched_set_fifo\0\0"
	"\x14\x00\x00\x00\x32\x48\x38\xb9"
	"single_open\0"
	"\x10\x00\x00\x00\x57\x9e\x04\x05"
	"put_pid\0"
	"\x1c\x00\x00\x00\xfd\x5e\x46\x1d"
	"kmem_cache_destroy\0\0"
	"\x18\x00\x00\x00\x31\x03\xb4\x32"
	"module_layout\0\0\0"
	"\x00\x00\x00\x00\x00\x00\x00\x00";

MODULE_INFO(depends, "");


MODULE_INFO(srcversion, "7DA134CB0FAFCF0FCC01581");
