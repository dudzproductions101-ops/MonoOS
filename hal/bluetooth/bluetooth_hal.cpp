// bluetooth_hal.cpp – MonoOS Hardware Abstraction Layer: BLUETOOTH
//
// Provides the platform-agnostic bluetooth HAL interface consumed by the
// bluetooth_service system service.  Concrete implementations are loaded
// at runtime from /vendor/lib64/hw/bluetooth.default.so via dlopen().

#include <cstdio>
#include <cstring>
#include <dlfcn.h>
#include <memory>
#include <string>

namespace monoos::hal {

// ─────────────────────────────────────────────────────────────────────
//  IBluetoothHal – pure-virtual HAL interface
// ─────────────────────────────────────────────────────────────────────

class IBluetoothHal {
public:
    virtual ~IBluetoothHal() = default;
    virtual bool open()              = 0;
    virtual void close()             = 0;
    virtual bool is_supported() const = 0;
    virtual int  get_version() const  = 0;
    virtual bool self_test()          = 0;
};

// ─────────────────────────────────────────────────────────────────────
//  StubBluetoothHal – emulator / CI / test builds
// ─────────────────────────────────────────────────────────────────────

class StubBluetoothHal final : public IBluetoothHal {
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

static const char *VENDOR_LIB = "/vendor/lib64/hw/bluetooth.default.so";
using CreateFn = IBluetoothHal *(*)();

static std::unique_ptr<IBluetoothHal> load_vendor_hal()
{
    void *lib = dlopen(VENDOR_LIB, RTLD_NOW | RTLD_LOCAL);
    if (!lib) {
        fprintf(stderr, "[hal/bluetooth] vendor lib not found (%s) – using stub\n",
                dlerror());
        return std::make_unique<StubBluetoothHal>();
    }
    auto *fn = reinterpret_cast<CreateFn>(dlsym(lib, "monoos_hal_create"));
    if (!fn) {
        dlclose(lib);
        return std::make_unique<StubBluetoothHal>();
    }
    return std::unique_ptr<IBluetoothHal>(fn());
}

// ─────────────────────────────────────────────────────────────────────
//  BluetoothHalManager – singleton that owns the implementation
// ─────────────────────────────────────────────────────────────────────

class BluetoothHalManager {
public:
    static BluetoothHalManager &instance() {
        static BluetoothHalManager mgr;
        return mgr;
    }

    bool init() {
        hal_ = load_vendor_hal();
        if (!hal_) return false;
        bool ok = hal_->open();
        fprintf(stderr, "[hal/bluetooth] init %s (v%d)\n",
                ok ? "OK" : "FAILED", hal_->get_version());
        return ok;
    }

    IBluetoothHal *get() const { return hal_.get(); }

private:
    std::unique_ptr<IBluetoothHal> hal_;
};

} // namespace monoos::hal

// ─────────────────────────────────────────────────────────────────────
//  C API consumed by the bluetooth_service
// ─────────────────────────────────────────────────────────────────────

extern "C" {

int monoos_bluetooth_hal_init() {
    return monoos::hal::BluetoothHalManager::instance().init() ? 0 : -1;
}

int monoos_bluetooth_hal_self_test() {
    auto *h = monoos::hal::BluetoothHalManager::instance().get();
    return (h && h->self_test()) ? 0 : -1;
}

int monoos_bluetooth_hal_version() {
    auto *h = monoos::hal::BluetoothHalManager::instance().get();
    return h ? h->get_version() : -1;
}

} // extern "C"
