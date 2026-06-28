/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _MONOOS_SCHED_H
#define _MONOOS_SCHED_H

#include <linux/types.h>

/** Classification of an app thread for scheduling purposes. */
typedef enum {
    THREAD_CLASS_UNKNOWN    = 0,
    THREAD_CLASS_FOREGROUND = 1,
    THREAD_CLASS_RENDER     = 2,
    THREAD_CLASS_AUDIO      = 3,
    THREAD_CLASS_BACKGROUND = 4,
    THREAD_CLASS_IDLE       = 5,
} thread_class_t;

int  monoos_sched_classify(pid_t tid, thread_class_t cls);
int  monoos_sched_set_frame_target(pid_t tid, u32 fps);
void monoos_sched_frame_begin(pid_t tid);
void monoos_sched_unregister(pid_t tid);

#endif /* _MONOOS_SCHED_H */
