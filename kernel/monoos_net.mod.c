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

KSYMTAB_FUNC(monoos_net_set_rule, "_gpl", "");

SYMBOL_CRC(monoos_net_set_rule, 0xa141e29c, "_gpl");

static const char ____versions[]
__used __section("__versions") =
	"\x1c\x00\x00\x00\x48\x9f\xdb\x88"
	"__check_object_size\0"
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
	"\x18\x00\x00\x00\x4e\xba\x82\x76"
	"__copy_overflow\0"
	"\x20\x00\x00\x00\x8b\x51\xa9\x3c"
	"nf_register_net_hooks\0\0\0"
	"\x28\x00\x00\x00\xb3\x1c\xa2\x87"
	"__ubsan_handle_out_of_bounds\0\0\0\0"
	"\x14\x00\x00\x00\x67\xc5\xdb\x48"
	"init_net\0\0\0\0"
	"\x1c\x00\x00\x00\xb6\x55\x4d\xdd"
	"_raw_read_unlock\0\0\0\0"
	"\x20\x00\x00\x00\x1b\xfa\xf1\x90"
	"nf_unregister_net_hooks\0"
	"\x24\x00\x00\x00\x70\xce\x5c\xd3"
	"_raw_spin_unlock_irqrestore\0"
	"\x14\x00\x00\x00\x96\x01\xb0\x91"
	"proc_mkdir\0\0"
	"\x1c\x00\x00\x00\xca\x39\x82\x5b"
	"__x86_return_thunk\0\0"
	"\x18\x00\x00\x00\xe1\xbe\x10\x6b"
	"_copy_to_user\0\0\0"
	"\x14\x00\x00\x00\x73\x10\x33\xfe"
	"proc_remove\0"
	"\x28\x00\x00\x00\xee\x8a\x07\xeb"
	"_raw_write_unlock_irqrestore\0\0\0\0"
	"\x14\x00\x00\x00\x32\x5b\xc6\x7c"
	"seq_read\0\0\0\0"
	"\x18\x00\x00\x00\x23\x61\xd6\x2c"
	"init_user_ns\0\0\0\0"
	"\x14\x00\x00\x00\x65\x93\x3f\xb4"
	"ktime_get\0\0\0"
	"\x14\x00\x00\x00\x10\xd3\x2e\x16"
	"seq_printf\0\0"
	"\x18\x00\x00\x00\xf0\x61\x8c\xfe"
	"_raw_read_lock\0\0"
	"\x18\x00\x00\x00\x9f\xbe\xbe\xd3"
	"single_release\0\0"
	"\x1c\x00\x00\x00\x69\xa8\x66\x14"
	"from_kuid_munged\0\0\0\0"
	"\x20\x00\x00\x00\x81\xbd\x21\x50"
	"_raw_write_lock_irqsave\0"
	"\x2c\x00\x00\x00\xc6\xfa\xb1\x54"
	"__ubsan_handle_load_invalid_value\0\0\0"
	"\x14\x00\x00\x00\x32\x48\x38\xb9"
	"single_open\0"
	"\x14\x00\x00\x00\xf7\xad\xd0\x13"
	"__kfifo_out\0"
	"\x18\x00\x00\x00\x31\x03\xb4\x32"
	"module_layout\0\0\0"
	"\x00\x00\x00\x00\x00\x00\x00\x00";

MODULE_INFO(depends, "");


MODULE_INFO(srcversion, "2C7ED1B76404EB45C75A083");
