/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _MONOOS_POWER_H
#define _MONOOS_POWER_H

#include <linux/types.h>
#include <linux/notifier.h>

int  monoos_wakelock_acquire(const char *name, uid_t uid);
int  monoos_wakelock_release(const char *name, uid_t uid);
void monoos_power_charge_cpu_ns(uid_t uid, u64 ns);
void monoos_power_charge_network(uid_t uid, u64 tx, u64 rx);
int  monoos_register_screen_notifier(struct notifier_block *nb);
int  monoos_unregister_screen_notifier(struct notifier_block *nb);

#endif /* _MONOOS_POWER_H */
