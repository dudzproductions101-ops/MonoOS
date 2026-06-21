// camera_hal.cpp – MonoOS Hardware Abstraction Layer: CAMERA
//
// Provides the platform-agnostic camera HAL interface consumed by the
// camera_service system service.  Concrete implementations are loaded
// at runtime from /vendor/lib64/hw/camera.default.so via dlopen().

#include <cstdio>
#include <cstring>
#include <dlfcn.h>
#include <memory>
#include <string>

namespace monoos::hal {

// ─────────────────────────────────────────────────────────────────────
//  ICameraHal – pure-virtual HAL interface
// ─────────────────────────────────────────────────────────────────────

class ICameraHal {
public:
    virtual ~ICameraHal() = default;
    virtual bool open()              = 0;
    virtual void close()             = 0;
    virtual bool is_supported() const = 0;
    virtual int  get_version() const  = 0;
    virtual bool self_test()          = 0;
};

// ─────────────────────────────────────────────────────────────────────
//  StubCameraHal – emulator / CI / test builds
// ─────────────────────────────────────────────────────────────────────

class StubCameraHal final : public ICameraHal {
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

static const char *VENDOR_LIB = "/vendor/lib64/hw/camera.default.so";
using CreateFn = ICameraHal *(*)();

static std::unique_ptr<ICameraHal> load_vendor_hal()
{
    void *lib = dlopen(VENDOR_LIB, RTLD_NOW | RTLD_LOCAL);
    if (!lib) {
        fprintf(stderr, "[hal/camera] vendor lib not found (%s) – using stub\n",
                dlerror());
        return std::make_unique<StubCameraHal>();
    }
    auto *fn = reinterpret_cast<CreateFn>(dlsym(lib, "monoos_hal_create"));
    if (!fn) {
        dlclose(lib);
        return std::make_unique<StubCameraHal>();
    }
    return std::unique_ptr<ICameraHal>(fn());
}

// ─────────────────────────────────────────────────────────────────────
//  CameraHalManager – singleton that owns the implementation
// ─────────────────────────────────────────────────────────────────────

class CameraHalManager {
public:
    static CameraHalManager &instance() {
        static CameraHalManager mgr;
        return mgr;
    }

    bool init() {
        hal_ = load_vendor_hal();
        if (!hal_) return false;
        bool ok = hal_->open();
        fprintf(stderr, "[hal/camera] init %s (v%d)\n",
                ok ? "OK" : "FAILED", hal_->get_version());
        return ok;
    }

    ICameraHal *get() const { return hal_.get(); }

private:
    std::unique_ptr<ICameraHal> hal_;
};

} // namespace monoos::hal

// ─────────────────────────────────────────────────────────────────────
//  C API consumed by the camera_service
// ─────────────────────────────────────────────────────────────────────

extern "C" {

int monoos_camera_hal_init() {
    return monoos::hal::CameraHalManager::instance().init() ? 0 : -1;
}

int monoos_camera_hal_self_test() {
    auto *h = monoos::hal::CameraHalManager::instance().get();
    return (h && h->self_test()) ? 0 : -1;
}

int monoos_camera_hal_version() {
    auto *h = monoos::hal::CameraHalManager::instance().get();
    return h ? h->get_version() : -1;
}

} // extern "C"
