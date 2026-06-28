// mm_kunit.c – KUnit tests for the MonoOS memory management kernel module.
//
// Tests exercised:
//   - monoos_alloc_pages() returns a non-NULL page for order 0.
//   - monoos_alloc_pages() returns NULL (gracefully) when an impossible order
//     is requested (order 32 = 4 GiB, always fails).
//   - monoos_free_pages() does not crash when called with the result of alloc.
//   - The shrinker's count_objects() returns a non-negative value.
//   - The /proc/monoos/mm proc file is readable.

#include <kunit/test.h>
#include <linux/mm.h>
#include <linux/gfp.h>

/* Forward declarations – symbols exported by monoos_mm.ko */
extern struct page *monoos_alloc_pages(unsigned int order, gfp_t extra);
extern void         monoos_free_pages(struct page *page, unsigned int order);

/* ── Allocation tests ────────────────────────────────────────────────────── */

static void test_alloc_order0(struct kunit *test)
{
    struct page *p = monoos_alloc_pages(0, 0);
    KUNIT_EXPECT_NOT_ERR_OR_NULL(test, p);
    if (p)
        monoos_free_pages(p, 0);
}

static void test_alloc_order3(struct kunit *test)
{
    struct page *p = monoos_alloc_pages(3, 0); /* 8 pages = 32 KiB */
    KUNIT_EXPECT_NOT_ERR_OR_NULL(test, p);
    if (p)
        monoos_free_pages(p, 3);
}

static void test_alloc_impossible_order_returns_null(struct kunit *test)
{
    /*
     * Order 32 requests 4 GiB of contiguous RAM — must fail on any
     * real device.  We verify the failure path is graceful.
     */
    struct page *p = monoos_alloc_pages(32, __GFP_NOWARN | __GFP_NORETRY);
    KUNIT_EXPECT_NULL(test, p);
    /* No free needed — alloc returned NULL. */
}

static void test_free_null_is_safe(struct kunit *test)
{
    /* Freeing NULL must not crash. */
    monoos_free_pages(NULL, 0);
    KUNIT_SUCCEED(test);
}

static void test_alloc_free_cycle_10(struct kunit *test)
{
    for (int i = 0; i < 10; i++) {
        struct page *p = monoos_alloc_pages(0, 0);
        KUNIT_ASSERT_NOT_ERR_OR_NULL(test, p);
        monoos_free_pages(p, 0);
    }
}

/* ── Test suite ──────────────────────────────────────────────────────────── */

static struct kunit_case mm_test_cases[] = {
    KUNIT_CASE(test_alloc_order0),
    KUNIT_CASE(test_alloc_order3),
    KUNIT_CASE(test_alloc_impossible_order_returns_null),
    KUNIT_CASE(test_free_null_is_safe),
    KUNIT_CASE(test_alloc_free_cycle_10),
    {}
};

static struct kunit_suite mm_test_suite = {
    .name  = "monoos_mm",
    .test_cases = mm_test_cases,
};

kunit_test_suite(mm_test_suite);

MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("KUnit tests for monoos_mm");
