/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _MONOOS_PROCESS_H
#define _MONOOS_PROCESS_H

#include <linux/types.h>

int  monoos_proc_register(pid_t pid, uid_t uid, const char *comm);
void monoos_proc_unregister(pid_t pid);
int  monoos_proc_grant_perm(pid_t pid, u32 perm_bit);
int  monoos_proc_revoke_perm(pid_t pid, u32 perm_bit);
bool monoos_proc_has_perm(pid_t pid, u32 perm_bit);

#endif /* _MONOOS_PROCESS_H */
