// vulkan_context.cpp – MonoOS Vulkan context bootstrap
//
// Creates the minimal Vulkan instance + device needed by the render engine.
// On the real device this loads libvulkan.so and selects the Adreno / Mali GPU.

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <vector>
#include <string>

// Stub Vulkan types for host compilation.
#ifndef VK_VERSION_1_0
  typedef uint32_t VkResult;
  static constexpr VkResult VK_SUCCESS = 0;
  static constexpr VkResult VK_ERROR_INITIALIZATION_FAILED = -3;
  struct VkInstance_T{}; typedef VkInstance_T* VkInstance;
  struct VkPhysicalDevice_T{}; typedef VkPhysicalDevice_T* VkPhysicalDevice;
  struct VkDevice_T{}; typedef VkDevice_T* VkDevice;
  struct VkQueue_T{}; typedef VkQueue_T* VkQueue;
  struct VkApplicationInfo { uint32_t sType; const void* pNext; const char* pApplicationName;
    uint32_t applicationVersion; const char* pEngineName; uint32_t engineVersion; uint32_t apiVersion; };
  struct VkInstanceCreateInfo { uint32_t sType; const void* pNext; uint32_t flags;
    const VkApplicationInfo* pApplicationInfo; uint32_t enabledLayerCount;
    const char* const* ppEnabledLayerNames; uint32_t enabledExtensionCount;
    const char* const* ppEnabledExtensionNames; };
  static VkResult vkCreateInstance(const VkInstanceCreateInfo*, const void*, VkInstance* i)
    { static VkInstance_T inst; *i = &inst; return VK_SUCCESS; }
  static void vkDestroyInstance(VkInstance, const void*) {}
#endif

namespace monoos::gfx {

struct VulkanContextConfig {
    std::string app_name       = "MonoOS";
    uint32_t    app_version    = 1;
    bool        validation     = false;   // enable Vulkan validation layers
    bool        surface_ext    = true;    // require VK_KHR_surface
};

class VulkanContext {
public:
    static VulkanContext &instance() {
        static VulkanContext ctx;
        return ctx;
    }

    bool init(const VulkanContextConfig &cfg = {}) {
        cfg_ = cfg;

        const char *layers[] = { "VK_LAYER_KHRONOS_validation" };
        const char *exts[]   = { "VK_KHR_surface", "VK_KHR_android_surface" };

        VkApplicationInfo app_info{};
        app_info.sType            = 0; // VK_STRUCTURE_TYPE_APPLICATION_INFO
        app_info.pApplicationName = cfg_.app_name.c_str();
        app_info.applicationVersion = cfg_.app_version;
        app_info.pEngineName      = "MonoOS RenderEngine";
        app_info.engineVersion    = 1;
        app_info.apiVersion       = 0x00401000; // VK 1.1

        VkInstanceCreateInfo ci{};
        ci.sType                   = 0; // VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO
        ci.pApplicationInfo        = &app_info;
        ci.enabledLayerCount       = cfg_.validation ? 1 : 0;
        ci.ppEnabledLayerNames     = cfg_.validation ? layers : nullptr;
        ci.enabledExtensionCount   = cfg_.surface_ext ? 2 : 0;
        ci.ppEnabledExtensionNames = cfg_.surface_ext ? exts : nullptr;

        VkResult res = vkCreateInstance(&ci, nullptr, &instance_);
        if (res != VK_SUCCESS) {
            fprintf(stderr, "[vulkan] vkCreateInstance failed: %d\n", (int)res);
            return false;
        }

        // On the real device: enumerate physical devices, pick GPU, create
        // logical device, get graphics + present queues.
        // Stub: report success.
        ready_ = true;
        fprintf(stderr, "[vulkan] context initialised (validation=%s)\n",
                cfg_.validation ? "ON" : "OFF");
        return true;
    }

    void shutdown() {
        if (ready_) { vkDestroyInstance(instance_, nullptr); ready_ = false; }
    }

    bool is_ready()      const { return ready_; }
    VkInstance vk_instance() const { return instance_; }

private:
    VulkanContext() = default;
    VkInstance          instance_ = nullptr;
    VulkanContextConfig cfg_;
    bool                ready_    = false;
};

} // namespace monoos::gfx

extern "C" {
    int  monoos_vulkan_init(int validation) {
        monoos::gfx::VulkanContextConfig cfg;
        cfg.validation = (validation != 0);
        return monoos::gfx::VulkanContext::instance().init(cfg) ? 0 : -1;
    }
    void monoos_vulkan_shutdown() { monoos::gfx::VulkanContext::instance().shutdown(); }
    int  monoos_vulkan_ready()    { return monoos::gfx::VulkanContext::instance().is_ready() ? 1 : 0; }
}
