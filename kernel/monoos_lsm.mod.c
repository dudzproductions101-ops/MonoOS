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



static const char ____versions[]
__used __section("__versions") =
	"\x1c\x00\x00\x00\x48\x9f\xdb\x88"
	"__check_object_size\0"
	"\x18\x00\x00\x00\x14\x27\x52\x8d"
	"__rcu_read_lock\0"
	"\x14\x00\x00\x00\xfd\x16\x99\xce"
	"proc_create\0"
	"\x14\x00\x00\x00\x6e\x4a\x6e\x65"
	"snprintf\0\0\0\0"
	"\x14\x00\x00\x00\x99\xcb\x3f\xf2"
	"__kfifo_in\0\0"
	"\x14\x00\x00\x00\xb8\x0f\xe5\xa4"
	"pcpu_hot\0\0\0\0"
	"\x14\x00\x00\x00\xed\x38\xe1\x0a"
	"seq_lseek\0\0\0"
	"\x20\x00\x00\x00\x0b\x05\xdb\x34"
	"_raw_spin_lock_irqsave\0\0"
	"\x14\x00\x00\x00\xbb\x6d\xfb\xbd"
	"__fentry__\0\0"
	"\x10\x00\x00\x00\x7e\x3a\x2c\x12"
	"_printk\0"
	"\x1c\x00\x00\x00\xcb\xf6\xfd\xf0"
	"__stack_chk_fail\0\0\0\0"
	"\x1c\x00\x00\x00\x0f\x81\x69\x24"
	"__rcu_read_unlock\0\0\0"
	"\x10\x00\x00\x00\x11\x13\x92\x5a"
	"strncmp\0"
	"\x20\x00\x00\x00\x87\x7d\xb7\x87"
	"unregister_kretprobe\0\0\0\0"
	"\x20\x00\x00\x00\x1e\x1e\x7e\x78"
	"monoos_proc_has_perm\0\0\0\0"
	"\x24\x00\x00\x00\x70\xce\x5c\xd3"
	"_raw_spin_unlock_irqrestore\0"
	"\x14\x00\x00\x00\x96\x01\xb0\x91"
	"proc_mkdir\0\0"
	"\x18\x00\x00\x00\x4f\xcf\x56\x03"
	"__get_task_comm\0"
	"\x1c\x00\x00\x00\xca\x39\x82\x5b"
	"__x86_return_thunk\0\0"
	"\x18\x00\x00\x00\xe1\xbe\x10\x6b"
	"_copy_to_user\0\0\0"
	"\x14\x00\x00\x00\x73\x10\x33\xfe"
	"proc_remove\0"
	"\x14\x00\x00\x00\x32\x5b\xc6\x7c"
	"seq_read\0\0\0\0"
	"\x14\x00\x00\x00\x65\x93\x3f\xb4"
	"ktime_get\0\0\0"
	"\x14\x00\x00\x00\x10\xd3\x2e\x16"
	"seq_printf\0\0"
	"\x1c\x00\x00\x00\xc0\x66\x48\x98"
	"register_kretprobe\0\0"
	"\x18\x00\x00\x00\x9f\xbe\xbe\xd3"
	"single_release\0\0"
	"\x1c\x00\x00\x00\x69\xa8\x66\x14"
	"from_kuid_munged\0\0\0\0"
	"\x14\x00\x00\x00\x32\x48\x38\xb9"
	"single_open\0"
	"\x14\x00\x00\x00\xf7\xad\xd0\x13"
	"__kfifo_out\0"
	"\x18\x00\x00\x00\x31\x03\xb4\x32"
	"module_layout\0\0\0"
	"\x00\x00\x00\x00\x00\x00\x00\x00";

MODULE_INFO(depends, "monoos_process");


MODULE_INFO(srcversion, "6C89DECA5B2D2439E253DF1");
