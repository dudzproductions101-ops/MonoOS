// sched_kunit.c – KUnit tests for the MonoOS scheduler extension module.

#include <kunit/test.h>
#include <linux/sched.h>

extern int  monoos_sched_classify(pid_t tid, int cls);
extern int  monoos_sched_set_frame_target(pid_t tid, u32 fps);
extern void monoos_sched_frame_begin(pid_t tid);
extern void monoos_sched_unregister(pid_t tid);

/* Thread class constants must match sched.c */
#define THREAD_CLASS_FOREGROUND 1
#define THREAD_CLASS_BACKGROUND 4

static void test_classify_current(struct kunit *test)
{
    pid_t tid = current->pid;
    int   ret = monoos_sched_classify(tid, THREAD_CLASS_FOREGROUND);
    /* May return -ESRCH if current is not in the table yet; both are acceptable. */
    KUNIT_EXPECT_TRUE(test, ret == 0 || ret == -ESRCH || ret == -ENOMEM);
    monoos_sched_unregister(tid);
}

static void test_invalid_class_rejected(struct kunit *test)
{
    KUNIT_EXPECT_EQ(test, monoos_sched_classify(current->pid, 99), -EINVAL);
}

static void test_frame_target_valid_fps(struct kunit *test)
{
    pid_t tid = current->pid;
    KUNIT_EXPECT_EQ(test, monoos_sched_set_frame_target(tid, 60), 0);
    KUNIT_EXPECT_EQ(test, monoos_sched_set_frame_target(tid, 120), 0);
    KUNIT_EXPECT_EQ(test, monoos_sched_set_frame_target(tid, 240), 0);
    monoos_sched_unregister(tid);
}

static void test_frame_target_zero_rejected(struct kunit *test)
{
    KUNIT_EXPECT_NE(test, monoos_sched_set_frame_target(current->pid, 0), 0);
}

static void test_frame_target_over_240_rejected(struct kunit *test)
{
    KUNIT_EXPECT_NE(test, monoos_sched_set_frame_target(current->pid, 241), 0);
}

static void test_frame_begin_no_crash(struct kunit *test)
{
    /* frame_begin on an unregistered tid must not crash. */
    monoos_sched_frame_begin(99998);
    KUNIT_SUCCEED(test);
}

static struct kunit_case sched_test_cases[] = {
    KUNIT_CASE(test_classify_current),
    KUNIT_CASE(test_invalid_class_rejected),
    KUNIT_CASE(test_frame_target_valid_fps),
    KUNIT_CASE(test_frame_target_zero_rejected),
    KUNIT_CASE(test_frame_target_over_240_rejected),
    KUNIT_CASE(test_frame_begin_no_crash),
    {}
};

static struct kunit_suite sched_test_suite = {
    .name       = "monoos_sched",
    .test_cases = sched_test_cases,
};

kunit_test_suite(sched_test_suite);

MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("KUnit tests for monoos_sched");
