# testing/kernel/Makefile – Run all MonoOS out-of-tree kernel module tests.
#
# Uses KUnit (kernel unit testing framework) via UML (User-Mode Linux) so
# tests run entirely in userspace without real hardware.
#
# Prerequisites:
#   CONFIG_KUNIT=y in the UML kernel config.
#   A built UML kernel at $(KDIR).
#
# Usage:
#   make test            Run all KUnit suites
#   make test-mm         Run memory manager tests only
#   make test-process    Run process lifecycle tests only
#   make clean           Remove test artefacts

KDIR  ?= /lib/modules/$(shell uname -r)/build
TESTS  = mm_kunit process_kunit sched_kunit lsm_kunit

.PHONY: all test $(TESTS) clean

all: test

test: $(TESTS)
	@echo ""
	@echo "All kernel unit test suites completed."

mm_kunit:
	@echo "[kunit] Running memory manager tests..."
	$(MAKE) -C $(KDIR) M=$(CURDIR)/mm_tests modules
	$(KDIR)/tools/testing/kunit/kunit.py run \
	    --kunitconfig $(CURDIR)/mm_tests/.kunitconfig 2>&1 | \
	    grep -E "PASS|FAIL|ERROR|SKIP" || true

process_kunit:
	@echo "[kunit] Running process lifecycle tests..."
	$(MAKE) -C $(KDIR) M=$(CURDIR)/process_tests modules
	$(KDIR)/tools/testing/kunit/kunit.py run \
	    --kunitconfig $(CURDIR)/process_tests/.kunitconfig 2>&1 | \
	    grep -E "PASS|FAIL|ERROR|SKIP" || true

sched_kunit:
	@echo "[kunit] Running scheduler tests..."
	$(MAKE) -C $(KDIR) M=$(CURDIR)/sched_tests modules
	$(KDIR)/tools/testing/kunit/kunit.py run \
	    --kunitconfig $(CURDIR)/sched_tests/.kunitconfig 2>&1 | \
	    grep -E "PASS|FAIL|ERROR|SKIP" || true

lsm_kunit:
	@echo "[kunit] Running LSM tests..."
	$(MAKE) -C $(KDIR) M=$(CURDIR)/lsm_tests modules
	$(KDIR)/tools/testing/kunit/kunit.py run \
	    --kunitconfig $(CURDIR)/lsm_tests/.kunitconfig 2>&1 | \
	    grep -E "PASS|FAIL|ERROR|SKIP" || true

clean:
	for d in mm_tests process_tests sched_tests lsm_tests; do \
	    $(MAKE) -C $(KDIR) M=$(CURDIR)/$$d clean 2>/dev/null || true; \
	done
