// display_hal.cpp – MonoOS Hardware Abstraction Layer: DISPLAY
//
// Provides the platform-agnostic display HAL interface consumed by the
// display_service system service.  Concrete implementations are loaded
// at runtime from /vendor/lib64/hw/display.default.so via dlopen().

#include <cstdio>
#include <cstring>
#include <dlfcn.h>
#include <memory>
#include <string>

namespace monoos::hal {

// ─────────────────────────────────────────────────────────────────────
//  IDisplayHal – pure-virtual HAL interface
// ─────────────────────────────────────────────────────────────────────

class IDisplayHal {
public:
    virtual ~IDisplayHal() = default;
    virtual bool open()              = 0;
    virtual void close()             = 0;
    virtual bool is_supported() const = 0;
    virtual int  get_version() const  = 0;
    virtual bool self_test()          = 0;
};

// ─────────────────────────────────────────────────────────────────────
//  StubDisplayHal – emulator / CI / test builds
// ─────────────────────────────────────────────────────────────────────

class StubDisplayHal final : public IDisplayHal {
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

static const char *VENDOR_LIB = "/vendor/lib64/hw/display.default.so";
using CreateFn = IDisplayHal *(*)();

static std::unique_ptr<IDisplayHal> load_vendor_hal()
{
    void *lib = dlopen(VENDOR_LIB, RTLD_NOW | RTLD_LOCAL);
    if (!lib) {
        fprintf(stderr, "[hal/display] vendor lib not found (%s) – using stub\n",
                dlerror());
        return std::make_unique<StubDisplayHal>();
    }
    auto *fn = reinterpret_cast<CreateFn>(dlsym(lib, "monoos_hal_create"));
    if (!fn) {
        dlclose(lib);
        return std::make_unique<StubDisplayHal>();
    }
    return std::unique_ptr<IDisplayHal>(fn());
}

// ─────────────────────────────────────────────────────────────────────
//  DisplayHalManager – singleton that owns the implementation
// ─────────────────────────────────────────────────────────────────────

class DisplayHalManager {
public:
    static DisplayHalManager &instance() {
        static DisplayHalManager mgr;
        return mgr;
    }

    bool init() {
        hal_ = load_vendor_hal();
        if (!hal_) return false;
        bool ok = hal_->open();
        fprintf(stderr, "[hal/display] init %s (v%d)\n",
                ok ? "OK" : "FAILED", hal_->get_version());
        return ok;
    }

    IDisplayHal *get() const { return hal_.get(); }

private:
    std::unique_ptr<IDisplayHal> hal_;
};

} // namespace monoos::hal

// ─────────────────────────────────────────────────────────────────────
//  C API consumed by the display_service
// ─────────────────────────────────────────────────────────────────────

extern "C" {

int monoos_display_hal_init() {
    return monoos::hal::DisplayHalManager::instance().init() ? 0 : -1;
}

int monoos_display_hal_self_test() {
    auto *h = monoos::hal::DisplayHalManager::instance().get();
    return (h && h->self_test()) ? 0 : -1;
}

int monoos_display_hal_version() {
    auto *h = monoos::hal::DisplayHalManager::instance().get();
    return h ? h->get_version() : -1;
}

} // extern "C"
