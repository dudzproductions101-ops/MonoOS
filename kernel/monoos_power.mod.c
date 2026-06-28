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

KSYMTAB_FUNC(monoos_wakelock_acquire, "_gpl", "");
KSYMTAB_FUNC(monoos_wakelock_release, "_gpl", "");
KSYMTAB_FUNC(monoos_power_charge_cpu_ns, "_gpl", "");
KSYMTAB_FUNC(monoos_power_charge_network, "_gpl", "");
KSYMTAB_FUNC(monoos_register_screen_notifier, "_gpl", "");
KSYMTAB_FUNC(monoos_unregister_screen_notifier, "_gpl", "");

SYMBOL_CRC(monoos_wakelock_acquire, 0x3e45e562, "_gpl");
SYMBOL_CRC(monoos_wakelock_release, 0x665c990e, "_gpl");
SYMBOL_CRC(monoos_power_charge_cpu_ns, 0x312c4a4c, "_gpl");
SYMBOL_CRC(monoos_power_charge_network, 0xb16466f3, "_gpl");
SYMBOL_CRC(monoos_register_screen_notifier, 0x230f353d, "_gpl");
SYMBOL_CRC(monoos_unregister_screen_notifier, 0x6151ee80, "_gpl");

static const char ____versions[]
__used __section("__versions") =
	"\x28\x00\x00\x00\xba\x24\x65\xd0"
	"raw_notifier_chain_unregister\0\0\0"
	"\x20\x00\x00\x00\x3f\x9d\x88\xd7"
	"wakeup_source_register\0\0"
	"\x20\x00\x00\x00\x3a\x97\x51\xa8"
	"raw_notifier_call_chain\0"
	"\x18\x00\x00\x00\x14\x27\x52\x8d"
	"__rcu_read_lock\0"
	"\x14\x00\x00\x00\xfd\x16\x99\xce"
	"proc_create\0"
	"\x20\x00\x00\x00\xcb\xba\x2c\xb1"
	"fb_unregister_client\0\0\0\0"
	"\x14\x00\x00\x00\xed\x38\xe1\x0a"
	"seq_lseek\0\0\0"
	"\x1c\x00\x00\x00\xcb\x60\x33\x14"
	"kmem_cache_create\0\0\0"
	"\x20\x00\x00\x00\x0b\x05\xdb\x34"
	"_raw_spin_lock_irqsave\0\0"
	"\x24\x00\x00\x00\x32\xa2\x9f\xdc"
	"raw_notifier_chain_register\0"
	"\x18\x00\x00\x00\x8c\x89\xd4\xcb"
	"fortify_panic\0\0\0"
	"\x14\x00\x00\x00\xbb\x6d\xfb\xbd"
	"__fentry__\0\0"
	"\x10\x00\x00\x00\x7e\x3a\x2c\x12"
	"_printk\0"
	"\x1c\x00\x00\x00\x7b\xcc\x27\x84"
	"_raw_spin_lock_irq\0\0"
	"\x10\x00\x00\x00\x94\xb6\x16\xa9"
	"strnlen\0"
	"\x1c\x00\x00\x00\x44\x6a\x71\x11"
	"kmem_cache_alloc\0\0\0\0"
	"\x28\x00\x00\x00\xb3\x1c\xa2\x87"
	"__ubsan_handle_out_of_bounds\0\0\0\0"
	"\x14\x00\x00\x00\x82\x3f\xfa\xae"
	"__pm_relax\0\0"
	"\x24\x00\x00\x00\xaa\x30\x77\xfa"
	"wakeup_source_unregister\0\0\0\0"
	"\x1c\x00\x00\x00\x0f\x81\x69\x24"
	"__rcu_read_unlock\0\0\0"
	"\x18\x00\x00\x00\x57\x0f\x44\x4b"
	"kmem_cache_free\0"
	"\x10\x00\x00\x00\x11\x13\x92\x5a"
	"strncmp\0"
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
	"\x10\x00\x00\x00\x39\xe6\x64\xdd"
	"strscpy\0"
	"\x14\x00\x00\x00\x65\x93\x3f\xb4"
	"ktime_get\0\0\0"
	"\x14\x00\x00\x00\x10\xd3\x2e\x16"
	"seq_printf\0\0"
	"\x14\x00\x00\x00\x26\x60\x86\x30"
	"seq_puts\0\0\0\0"
	"\x18\x00\x00\x00\x9f\xbe\xbe\xd3"
	"single_release\0\0"
	"\x18\x00\x00\x00\x40\x3c\x75\x1a"
	"__pm_stay_awake\0"
	"\x2c\x00\x00\x00\xc6\xfa\xb1\x54"
	"__ubsan_handle_load_invalid_value\0\0\0"
	"\x1c\x00\x00\x00\x3e\x8d\xd6\x80"
	"fb_register_client\0\0"
	"\x14\x00\x00\x00\x32\x48\x38\xb9"
	"single_open\0"
	"\x1c\x00\x00\x00\xfd\x5e\x46\x1d"
	"kmem_cache_destroy\0\0"
	"\x18\x00\x00\x00\x31\x03\xb4\x32"
	"module_layout\0\0\0"
	"\x00\x00\x00\x00\x00\x00\x00\x00";

MODULE_INFO(depends, "");


MODULE_INFO(srcversion, "00787D5DA21AFD7795A7181");
