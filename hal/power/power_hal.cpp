// power_hal.cpp – OneOS Hardware Abstraction Layer: POWER
//
// Provides the platform-agnostic power HAL interface consumed by the
// power_service system service.  Concrete implementations are loaded
// at runtime from /vendor/lib64/hw/power.default.so via dlopen().

#include <cstdio>
#include <cstring>
#include <dlfcn.h>
#include <memory>
#include <string>

namespace oneos::hal {

// ─────────────────────────────────────────────────────────────────────
//  IPowerHal – pure-virtual HAL interface
// ─────────────────────────────────────────────────────────────────────

class IPowerHal {
public:
    virtual ~IPowerHal() = default;
    virtual bool open()              = 0;
    virtual void close()             = 0;
    virtual bool is_supported() const = 0;
    virtual int  get_version() const  = 0;
    virtual bool self_test()          = 0;
};

// ─────────────────────────────────────────────────────────────────────
//  StubPowerHal – emulator / CI / test builds
// ─────────────────────────────────────────────────────────────────────

class StubPowerHal final : public IPowerHal {
    bool open_ = false;
public:
    bool open()              override { open_ = true;  return true; }
    void close()             override { open_ = false; }
    bool is_supported() const override { return true;  }
    int  get_version() const  override { return 1;     }
    bool self_test()          override { return open_;  }
};

// ─────────────────────────────────────────────────────────────────────
//  Vendor shared-library loader
// ─────────────────────────────────────────────────────────────────────

static const char *VENDOR_LIB = "/vendor/lib64/hw/power.default.so";
using CreateFn = IPowerHal *(*)();

static std::unique_ptr<IPowerHal> load_vendor_hal()
{
    void *lib = dlopen(VENDOR_LIB, RTLD_NOW | RTLD_LOCAL);
    if (!lib) {
        fprintf(stderr, "[hal/power] vendor lib not found (%s) – using stub\n",
                dlerror());
        return std::make_unique<StubPowerHal>();
    }
    auto *fn = reinterpret_cast<CreateFn>(dlsym(lib, "oneos_hal_create"));
    if (!fn) {
        dlclose(lib);
        return std::make_unique<StubPowerHal>();
    }
    return std::unique_ptr<IPowerHal>(fn());
}

// ─────────────────────────────────────────────────────────────────────
//  PowerHalManager – singleton that owns the implementation
// ─────────────────────────────────────────────────────────────────────

class PowerHalManager {
public:
    static PowerHalManager &instance() {
        static PowerHalManager mgr;
        return mgr;
    }

    bool init() {
        hal_ = load_vendor_hal();
        if (!hal_) return false;
        bool ok = hal_->open();
        fprintf(stderr, "[hal/power] init %s (v%d)\n",
                ok ? "OK" : "FAILED", hal_->get_version());
        return ok;
    }

    IPowerHal *get() const { return hal_.get(); }

private:
    std::unique_ptr<IPowerHal> hal_;
};

} // namespace oneos::hal

// ─────────────────────────────────────────────────────────────────────
//  C API consumed by the power_service
// ─────────────────────────────────────────────────────────────────────

extern "C" {

int oneos_power_hal_init() {
    return oneos::hal::PowerHalManager::instance().init() ? 0 : -1;
}

int oneos_power_hal_self_test() {
    auto *h = oneos::hal::PowerHalManager::instance().get();
    return (h && h->self_test()) ? 0 : -1;
}

int oneos_power_hal_version() {
    auto *h = oneos::hal::PowerHalManager::instance().get();
    return h ? h->get_version() : -1;
}

} // extern "C"
