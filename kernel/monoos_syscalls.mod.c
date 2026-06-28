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
	"\x1c\x00\x00\x00\x2b\x2f\xec\xe3"
	"alloc_chrdev_region\0"
	"\x18\x00\x00\x00\xc2\x9c\xc4\x13"
	"_copy_from_user\0"
	"\x18\x00\x00\x00\x54\x2c\x75\x76"
	"class_destroy\0\0\0"
	"\x14\x00\x00\x00\xb8\x0f\xe5\xa4"
	"pcpu_hot\0\0\0\0"
	"\x18\x00\x00\x00\x64\xbd\x8f\xba"
	"_raw_spin_lock\0\0"
	"\x14\x00\x00\x00\xbb\x6d\xfb\xbd"
	"__fentry__\0\0"
	"\x20\x00\x00\x00\x4e\xe5\x7c\x49"
	"monoos_proc_grant_perm\0\0"
	"\x10\x00\x00\x00\x7e\x3a\x2c\x12"
	"_printk\0"
	"\x1c\x00\x00\x00\xcb\xf6\xfd\xf0"
	"__stack_chk_fail\0\0\0\0"
	"\x10\x00\x00\x00\x89\xbc\xcb\xc6"
	"capable\0"
	"\x14\x00\x00\x00\x85\xc1\x7a\x0b"
	"cdev_add\0\0\0\0"
	"\x18\x00\x00\x00\x8b\xcf\x41\x4e"
	"device_create\0\0\0"
	"\x18\x00\x00\x00\x6f\xcd\x17\x23"
	"class_create\0\0\0\0"
	"\x20\x00\x00\x00\x1e\x1e\x7e\x78"
	"monoos_proc_has_perm\0\0\0\0"
	"\x1c\x00\x00\x00\xca\x39\x82\x5b"
	"__x86_return_thunk\0\0"
	"\x18\x00\x00\x00\xe1\xbe\x10\x6b"
	"_copy_to_user\0\0\0"
	"\x24\x00\x00\x00\x33\xb3\x91\x60"
	"unregister_chrdev_region\0\0\0\0"
	"\x20\x00\x00\x00\x7b\x1e\xb9\xa7"
	"monoos_proc_revoke_perm\0"
	"\x18\x00\x00\x00\x41\xc4\x9c\x6d"
	"device_destroy\0\0"
	"\x1c\x00\x00\x00\x69\xa8\x66\x14"
	"from_kuid_munged\0\0\0\0"
	"\x1c\x00\x00\x00\x34\x4b\xb5\xb5"
	"_raw_spin_unlock\0\0\0\0"
	"\x14\x00\x00\x00\xf1\xf4\x90\x60"
	"cdev_init\0\0\0"
	"\x14\x00\x00\x00\xfb\x53\xc0\xe5"
	"cdev_del\0\0\0\0"
	"\x18\x00\x00\x00\x31\x03\xb4\x32"
	"module_layout\0\0\0"
	"\x00\x00\x00\x00\x00\x00\x00\x00";

MODULE_INFO(depends, "monoos_process");


MODULE_INFO(srcversion, "E50F764E9270AB09C5F1160");
