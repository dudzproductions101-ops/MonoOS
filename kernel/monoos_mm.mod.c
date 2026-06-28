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

KSYMTAB_FUNC(monoos_alloc_pages, "_gpl", "");
KSYMTAB_FUNC(monoos_free_pages, "_gpl", "");
KSYMTAB_FUNC(monoos_vmalloc, "_gpl", "");

SYMBOL_CRC(monoos_alloc_pages, 0x9ea75646, "_gpl");
SYMBOL_CRC(monoos_free_pages, 0x7c617a13, "_gpl");
SYMBOL_CRC(monoos_vmalloc, 0x74b632b9, "_gpl");

static const char ____versions[]
__used __section("__versions") =
	"\x18\x00\x00\x00\x47\x8d\x36\xfd"
	"shrinker_alloc\0\0"
	"\x14\x00\x00\x00\xfd\x16\x99\xce"
	"proc_create\0"
	"\x14\x00\x00\x00\x2c\x01\x04\xae"
	"__vmalloc\0\0\0"
	"\x14\x00\x00\x00\xed\x38\xe1\x0a"
	"seq_lseek\0\0\0"
	"\x18\x00\x00\x00\x64\xbd\x8f\xba"
	"_raw_spin_lock\0\0"
	"\x14\x00\x00\x00\xbb\x6d\xfb\xbd"
	"__fentry__\0\0"
	"\x18\x00\x00\x00\xbe\x6e\xd3\x09"
	"shrinker_free\0\0\0"
	"\x10\x00\x00\x00\x7e\x3a\x2c\x12"
	"_printk\0"
	"\x1c\x00\x00\x00\x6f\xc7\x58\x82"
	"shrinker_register\0\0\0"
	"\x18\x00\x00\x00\xc5\xee\x10\x18"
	"__free_pages\0\0\0\0"
	"\x14\x00\x00\x00\x96\x01\xb0\x91"
	"proc_mkdir\0\0"
	"\x1c\x00\x00\x00\xca\x39\x82\x5b"
	"__x86_return_thunk\0\0"
	"\x14\x00\x00\x00\x73\x10\x33\xfe"
	"proc_remove\0"
	"\x14\x00\x00\x00\x32\x5b\xc6\x7c"
	"seq_read\0\0\0\0"
	"\x2c\x00\x00\x00\x61\xe5\x48\xa6"
	"__ubsan_handle_shift_out_of_bounds\0\0"
	"\x14\x00\x00\x00\x2c\xc4\x3c\xbd"
	"alloc_pages\0"
	"\x14\x00\x00\x00\x10\xd3\x2e\x16"
	"seq_printf\0\0"
	"\x18\x00\x00\x00\x9f\xbe\xbe\xd3"
	"single_release\0\0"
	"\x14\x00\x00\x00\x32\x48\x38\xb9"
	"single_open\0"
	"\x1c\x00\x00\x00\x34\x4b\xb5\xb5"
	"_raw_spin_unlock\0\0\0\0"
	"\x18\x00\x00\x00\x31\x03\xb4\x32"
	"module_layout\0\0\0"
	"\x00\x00\x00\x00\x00\x00\x00\x00";

MODULE_INFO(depends, "");


MODULE_INFO(srcversion, "C779225DF78A970D168EA5E");
