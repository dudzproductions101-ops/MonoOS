// process_kunit.c – KUnit tests for the MonoOS process lifecycle module.

#include <kunit/test.h>
#include <linux/sched.h>

/* Exported symbols from monoos_process.ko */
extern int  monoos_proc_register(pid_t pid, uid_t uid, const char *comm);
extern void monoos_proc_unregister(pid_t pid);
extern int  monoos_proc_grant_perm(pid_t pid, u32 perm_bit);
extern int  monoos_proc_revoke_perm(pid_t pid, u32 perm_bit);
extern bool monoos_proc_has_perm(pid_t pid, u32 perm_bit);

#define PERM_CAMERA  0x0001U
#define PERM_MIC     0x0002U

static void test_register_and_grant(struct kunit *test)
{
    const pid_t pid = 99001;
    KUNIT_ASSERT_EQ(test, monoos_proc_register(pid, 10001, "test_app"), 0);
    KUNIT_ASSERT_EQ(test, monoos_proc_grant_perm(pid, PERM_CAMERA), 0);
    KUNIT_EXPECT_TRUE(test, monoos_proc_has_perm(pid, PERM_CAMERA));
    KUNIT_EXPECT_FALSE(test, monoos_proc_has_perm(pid, PERM_MIC));
    monoos_proc_unregister(pid);
}

static void test_revoke(struct kunit *test)
{
    const pid_t pid = 99002;
    monoos_proc_register(pid, 10002, "test_revoke");
    monoos_proc_grant_perm(pid, PERM_MIC);
    KUNIT_EXPECT_TRUE(test, monoos_proc_has_perm(pid, PERM_MIC));
    monoos_proc_revoke_perm(pid, PERM_MIC);
    KUNIT_EXPECT_FALSE(test, monoos_proc_has_perm(pid, PERM_MIC));
    monoos_proc_unregister(pid);
}

static void test_unregistered_pid_returns_esrch(struct kunit *test)
{
    KUNIT_EXPECT_EQ(test, monoos_proc_grant_perm(99999, PERM_CAMERA), -ESRCH);
    KUNIT_EXPECT_FALSE(test, monoos_proc_has_perm(99999, PERM_CAMERA));
}

static void test_multiple_perms_bitmask(struct kunit *test)
{
    const pid_t pid = 99003;
    monoos_proc_register(pid, 10003, "multi_perm");
    monoos_proc_grant_perm(pid, PERM_CAMERA);
    monoos_proc_grant_perm(pid, PERM_MIC);
    KUNIT_EXPECT_TRUE(test, monoos_proc_has_perm(pid, PERM_CAMERA));
    KUNIT_EXPECT_TRUE(test, monoos_proc_has_perm(pid, PERM_MIC));
    monoos_proc_revoke_perm(pid, PERM_CAMERA);
    KUNIT_EXPECT_FALSE(test, monoos_proc_has_perm(pid, PERM_CAMERA));
    KUNIT_EXPECT_TRUE(test, monoos_proc_has_perm(pid, PERM_MIC));
    monoos_proc_unregister(pid);
}

static struct kunit_case process_test_cases[] = {
    KUNIT_CASE(test_register_and_grant),
    KUNIT_CASE(test_revoke),
    KUNIT_CASE(test_unregistered_pid_returns_esrch),
    KUNIT_CASE(test_multiple_perms_bitmask),
    {}
};

static struct kunit_suite process_test_suite = {
    .name       = "monoos_process",
    .test_cases = process_test_cases,
};

kunit_test_suite(process_test_suite);

MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("KUnit tests for monoos_process");
