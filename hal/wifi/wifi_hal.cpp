// wifi_hal.cpp – MonoOS Hardware Abstraction Layer: WIFI
//
// Provides the platform-agnostic wifi HAL interface consumed by the
// wifi_service system service.  Concrete implementations are loaded
// at runtime from /vendor/lib64/hw/wifi.default.so via dlopen().

#include <cstdio>
#include <cstring>
#include <dlfcn.h>
#include <memory>
#include <string>

namespace monoos::hal {

// ─────────────────────────────────────────────────────────────────────
//  IWifiHal – pure-virtual HAL interface
// ─────────────────────────────────────────────────────────────────────

class IWifiHal {
public:
    virtual ~IWifiHal() = default;
    virtual bool open()              = 0;
    virtual void close()             = 0;
    virtual bool is_supported() const = 0;
    virtual int  get_version() const  = 0;
    virtual bool self_test()          = 0;
};

// ─────────────────────────────────────────────────────────────────────
//  StubWifiHal – emulator / CI / test builds
// ─────────────────────────────────────────────────────────────────────

class StubWifiHal final : public IWifiHal {
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

static const char *VENDOR_LIB = "/vendor/lib64/hw/wifi.default.so";
using CreateFn = IWifiHal *(*)();

static std::unique_ptr<IWifiHal> load_vendor_hal()
{
    void *lib = dlopen(VENDOR_LIB, RTLD_NOW | RTLD_LOCAL);
    if (!lib) {
        fprintf(stderr, "[hal/wifi] vendor lib not found (%s) – using stub\n",
                dlerror());
        return std::make_unique<StubWifiHal>();
    }
    auto *fn = reinterpret_cast<CreateFn>(dlsym(lib, "monoos_hal_create"));
    if (!fn) {
        dlclose(lib);
        return std::make_unique<StubWifiHal>();
    }
    return std::unique_ptr<IWifiHal>(fn());
}

// ─────────────────────────────────────────────────────────────────────
//  WifiHalManager – singleton that owns the implementation
// ─────────────────────────────────────────────────────────────────────

class WifiHalManager {
public:
    static WifiHalManager &instance() {
        static WifiHalManager mgr;
        return mgr;
    }

    bool init() {
        hal_ = load_vendor_hal();
        if (!hal_) return false;
        bool ok = hal_->open();
        fprintf(stderr, "[hal/wifi] init %s (v%d)\n",
                ok ? "OK" : "FAILED", hal_->get_version());
        return ok;
    }

    IWifiHal *get() const { return hal_.get(); }

private:
    std::unique_ptr<IWifiHal> hal_;
};

} // namespace monoos::hal

// ─────────────────────────────────────────────────────────────────────
//  C API consumed by the wifi_service
// ─────────────────────────────────────────────────────────────────────

extern "C" {

int monoos_wifi_hal_init() {
    return monoos::hal::WifiHalManager::instance().init() ? 0 : -1;
}

int monoos_wifi_hal_self_test() {
    auto *h = monoos::hal::WifiHalManager::instance().get();
    return (h && h->self_test()) ? 0 : -1;
}

int monoos_wifi_hal_version() {
    auto *h = monoos::hal::WifiHalManager::instance().get();
    return h ? h->get_version() : -1;
}

} // extern "C"
