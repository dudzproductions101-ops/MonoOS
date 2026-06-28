/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _MONOOS_NET_H
#define _MONOOS_NET_H

#include <linux/types.h>

int monoos_net_set_rule(uid_t uid, bool allow, bool block_trackers,
                        bool vpn_only);

#endif /* _MONOOS_NET_H */
