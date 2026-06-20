// render_engine.cpp – OneOS Wayland/Vulkan Render Engine Framework Layer
//
// Sits between the SurfaceFlinger compositor and the Vulkan HAL.
// Responsibilities:
//   1. Manage a pool of Vulkan swap-chain images per output (display).
//   2. Accept layer compositing requests from SurfaceFlinger.
//   3. Drive the Vulkan render pass: bind pipeline, draw quads, present.
//   4. Implement frame pacing using the VK_EXT_present_timing extension.

#include <algorithm>
#include <array>
#include <cassert>
#include <cstdio>
#include <cstring>
#include <memory>
#include <vector>

// Forward-declare Vulkan types so this compiles without the Vulkan SDK
// on the build host; the real device build links against libvulkan.so.
struct VkInstance_T;   using VkInstance   = VkInstance_T*;
struct VkDevice_T;     using VkDevice     = VkDevice_T*;
struct VkSwapchainKHR_T; using VkSwapchainKHR = VkSwapchainKHR_T*;
struct VkImage_T;      using VkImage      = VkImage_T*;
struct VkSemaphore_T;  using VkSemaphore  = VkSemaphore_T*;
using VkResult = int;
constexpr VkResult VK_SUCCESS = 0;

namespace oneos::gfx {

// ──────────────────────────────────────────────────────────────────────────────
//  Layer descriptor — one per app window surface
// ──────────────────────────────────────────────────────────────────────────────

struct LayerDesc {
    uint32_t    z_order;      // higher = closer to user
    float       alpha;        // 0.0–1.0
    float       x, y;        // position in output coordinates
    float       width, height;
    VkImage     src_image;    // Vulkan image backing the app buffer
    bool        opaque;
    bool        secure;       // DRM-protected content
};

// ──────────────────────────────────────────────────────────────────────────────
//  FrameStats
// ──────────────────────────────────────────────────────────────────────────────

struct FrameStats {
    uint64_t frame_number       = 0;
    uint64_t present_time_ns    = 0;   // when frame was displayed
    uint64_t gpu_duration_ns    = 0;   // GPU render time
    uint32_t dropped_frames     = 0;
    float    fps                = 0.0f;
};

// ──────────────────────────────────────────────────────────────────────────────
//  RenderOutput — one per physical display
// ──────────────────────────────────────────────────────────────────────────────

class RenderOutput {
public:
    RenderOutput(uint32_t width, uint32_t height, uint32_t refresh_hz)
        : width_(width), height_(height), refresh_hz_(refresh_hz) {}

    uint32_t width()      const { return width_;      }
    uint32_t height()     const { return height_;     }
    uint32_t refresh_hz() const { return refresh_hz_; }

    uint64_t frame_period_ns() const {
        return refresh_hz_ > 0 ? 1'000'000'000ULL / refresh_hz_ : 16'666'667ULL;
    }

private:
    uint32_t width_, height_, refresh_hz_;
};

// ──────────────────────────────────────────────────────────────────────────────
//  RenderEngine
// ──────────────────────────────────────────────────────────────────────────────

class RenderEngine {
public:
    static RenderEngine &instance() {
        static RenderEngine engine;
        return engine;
    }

    /// Initialise Vulkan instance, device, and swap chain for the given output.
    bool init(VkInstance instance, VkDevice device,
              uint32_t display_w, uint32_t display_h, uint32_t refresh_hz)
    {
        vk_instance_ = instance;
        vk_device_   = device;
        output_ = std::make_unique<RenderOutput>(display_w, display_h, refresh_hz);
        fprintf(stderr, "[render_engine] init %ux%u @%uhz
",
                display_w, display_h, refresh_hz);
        return true;
    }

    /// Submit a frame: sort layers by z-order, composite, and present.
    bool present_frame(std::vector<LayerDesc> layers, uint64_t desired_present_ns)
    {
        if (!output_) return false;

        // Sort back-to-front.
        std::sort(layers.begin(), layers.end(),
                  [](const LayerDesc &a, const LayerDesc &b){
                      return a.z_order < b.z_order; });

        // --- begin Vulkan render pass ---
        // vkBeginCommandBuffer(cmd, …)
        // vkCmdBeginRenderPass(cmd, &rp_info, VK_SUBPASS_CONTENTS_INLINE)
        for (auto &layer : layers) {
            if (!layer.src_image) continue;   // transparent hole
            composite_layer(layer);
        }
        // vkCmdEndRenderPass(cmd)
        // vkEndCommandBuffer(cmd)
        // vkQueueSubmit(graphics_queue, …)
        // --- end Vulkan render pass ---

        // Present with timing hint.
        VkResult res = schedule_present(desired_present_ns);
        if (res != VK_SUCCESS) {
            ++stats_.dropped_frames;
            return false;
        }

        ++stats_.frame_number;
        update_fps();
        return true;
    }

    const FrameStats &stats() const { return stats_; }

    void shutdown() {
        // vkDestroySwapchainKHR, vkDestroyDevice, vkDestroyInstance…
        output_.reset();
        fprintf(stderr, "[render_engine] shutdown after %llu frames
",
                (unsigned long long)stats_.frame_number);
    }

private:
    RenderEngine() = default;

    void composite_layer(const LayerDesc &layer)
    {
        // Push constants: position, size, alpha.
        // vkCmdPushConstants(cmd, pipeline_layout, VK_SHADER_STAGE_FRAGMENT_BIT,
        //                    0, sizeof(PushConstants), &pc);
        // vkCmdBindDescriptorSets(…, layer.src_image descriptor …)
        // vkCmdDraw(cmd, 6, 1, 0, 0);  // fullscreen quad, 2 triangles
        (void)layer;
    }

    VkResult schedule_present(uint64_t desired_ns)
    {
        // VkPresentTimesInfoGOOGLE pt_info = { …, desired_ns };
        // vkQueuePresentKHR(present_queue, &present_info);
        (void)desired_ns;
        return VK_SUCCESS;
    }

    void update_fps()
    {
        static uint64_t last_ns = 0;
        static uint32_t frame_acc = 0;
        ++frame_acc;
        // real impl uses clock_gettime(CLOCK_MONOTONIC)
        if (frame_acc >= 60) {
            stats_.fps = 60.0f;   // placeholder
            frame_acc = 0;
            last_ns = 0;
        }
        (void)last_ns;
    }

    VkInstance                   vk_instance_ = nullptr;
    VkDevice                     vk_device_   = nullptr;
    VkSwapchainKHR               swapchain_   = nullptr;
    std::unique_ptr<RenderOutput> output_;
    FrameStats                   stats_;
};

} // namespace oneos::gfx

// C API used by SurfaceFlinger
extern "C" {
    int oneos_render_init(uint32_t w, uint32_t h, uint32_t hz) {
        return oneos::gfx::RenderEngine::instance().init(nullptr, nullptr, w, h, hz) ? 0 : -1;
    }
    void oneos_render_shutdown() {
        oneos::gfx::RenderEngine::instance().shutdown();
    }
}
