// sensors_hal.cpp – MonoOS Hardware Abstraction Layer: SENSORS
//
// Provides the platform-agnostic sensors HAL interface consumed by the
// sensors_service system service.  Concrete implementations are loaded
// at runtime from /vendor/lib64/hw/sensors.default.so via dlopen().

#include <cstdio>
#include <cstring>
#include <dlfcn.h>
#include <memory>
#include <string>

namespace monoos::hal {

// ─────────────────────────────────────────────────────────────────────
//  ISensorsHal – pure-virtual HAL interface
// ─────────────────────────────────────────────────────────────────────

class ISensorsHal {
public:
    virtual ~ISensorsHal() = default;
    virtual bool open()              = 0;
    virtual void close()             = 0;
    virtual bool is_supported() const = 0;
    virtual int  get_version() const  = 0;
    virtual bool self_test()          = 0;
};

// ─────────────────────────────────────────────────────────────────────
//  StubSensorsHal – emulator / CI / test builds
// ─────────────────────────────────────────────────────────────────────

class StubSensorsHal final : public ISensorsHal {
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

static const char *VENDOR_LIB = "/vendor/lib64/hw/sensors.default.so";
using CreateFn = ISensorsHal *(*)();

static std::unique_ptr<ISensorsHal> load_vendor_hal()
{
    void *lib = dlopen(VENDOR_LIB, RTLD_NOW | RTLD_LOCAL);
    if (!lib) {
        fprintf(stderr, "[hal/sensors] vendor lib not found (%s) – using stub\n",
                dlerror());
        return std::make_unique<StubSensorsHal>();
    }
    auto *fn = reinterpret_cast<CreateFn>(dlsym(lib, "monoos_hal_create"));
    if (!fn) {
        dlclose(lib);
        return std::make_unique<StubSensorsHal>();
    }
    return std::unique_ptr<ISensorsHal>(fn());
}

// ─────────────────────────────────────────────────────────────────────
//  SensorsHalManager – singleton that owns the implementation
// ─────────────────────────────────────────────────────────────────────

class SensorsHalManager {
public:
    static SensorsHalManager &instance() {
        static SensorsHalManager mgr;
        return mgr;
    }

    bool init() {
        hal_ = load_vendor_hal();
        if (!hal_) return false;
        bool ok = hal_->open();
        fprintf(stderr, "[hal/sensors] init %s (v%d)\n",
                ok ? "OK" : "FAILED", hal_->get_version());
        return ok;
    }

    ISensorsHal *get() const { return hal_.get(); }

private:
    std::unique_ptr<ISensorsHal> hal_;
};

} // namespace monoos::hal

// ─────────────────────────────────────────────────────────────────────
//  C API consumed by the sensors_service
// ─────────────────────────────────────────────────────────────────────

extern "C" {

int monoos_sensors_hal_init() {
    return monoos::hal::SensorsHalManager::instance().init() ? 0 : -1;
}

int monoos_sensors_hal_self_test() {
    auto *h = monoos::hal::SensorsHalManager::instance().get();
    return (h && h->self_test()) ? 0 : -1;
}

int monoos_sensors_hal_version() {
    auto *h = monoos::hal::SensorsHalManager::instance().get();
    return h ? h->get_version() : -1;
}

} // extern "C"
