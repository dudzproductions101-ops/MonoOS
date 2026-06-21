// camera_pipeline.cpp – MonoOS Camera Pipeline (V4L2 → ISP → Encoder)
//
// Manages the full camera capture path:
//   1. Open a V4L2 video device (/dev/videoN).
//   2. Negotiate format (YUYV / NV12) and buffer queue (MMAP).
//   3. Stream frames into a ring buffer.
//   4. Post-process via the ISP tuning pipeline (AWB, AE, AF stubs).
//   5. Hand off to the encoder or JPEG compressor for still capture.

#include <cerrno>
#include <cstring>
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <memory>
#include <string>
#include <vector>
#include <functional>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>

// V4L2 types — included when building on-device; stubbed here so this
// file compiles on the host without the kernel headers installed.
#ifndef V4L2_BUF_TYPE_VIDEO_CAPTURE
  typedef unsigned int __u32;
  typedef int          __s32;
  struct v4l2_fmtdesc { __u32 index, type, flags, pixelformat; char description[32]; __u32 reserved[4]; };
  struct v4l2_pix_format { __u32 width, height, pixelformat, field, bytesperline, sizeimage, colorspace; __u32 priv; };
  struct v4l2_format { __u32 type; union { struct v4l2_pix_format pix; char raw[200]; } fmt; };
  struct v4l2_requestbuffers { __u32 count, type, memory; __u32 reserved[2]; };
  struct v4l2_buffer { __u32 index, type, bytesused, flags, field; __u32 sequence; __u32 memory; union { __u32 offset; } m; __u32 length; };
  #define V4L2_BUF_TYPE_VIDEO_CAPTURE 1
  #define V4L2_MEMORY_MMAP            1
  #define V4L2_PIX_FMT_NV12   0x3231564e
  #define VIDIOC_S_FMT        _IOWR('V', 5, struct v4l2_format)
  #define VIDIOC_REQBUFS      _IOWR('V', 8, struct v4l2_requestbuffers)
  #define VIDIOC_QUERYBUF     _IOWR('V', 9, struct v4l2_buffer)
  #define VIDIOC_QBUF         _IOWR('V', 15, struct v4l2_buffer)
  #define VIDIOC_DQBUF        _IOWR('V', 17, struct v4l2_buffer)
  #define VIDIOC_STREAMON     _IOW ('V', 18, int)
  #define VIDIOC_STREAMOFF    _IOW ('V', 19, int)
#endif

namespace monoos::camera {

static constexpr int    NUM_BUFS    = 4;
static constexpr int    DEFAULT_W   = 1920;
static constexpr int    DEFAULT_H   = 1080;

// ─────────────────────────────────────────────────────────────────────────────
//  FrameBuffer – one mmap'd V4L2 buffer
// ─────────────────────────────────────────────────────────────────────────────
struct FrameBuffer {
    void   *start  = nullptr;
    size_t  length = 0;
};

// ─────────────────────────────────────────────────────────────────────────────
//  CameraFrame – what the pipeline delivers to consumers
// ─────────────────────────────────────────────────────────────────────────────
struct CameraFrame {
    const uint8_t *data;
    size_t         size;
    uint32_t       width, height;
    uint32_t       sequence;
    uint64_t       timestamp_ns;
};

using FrameCallback = std::function<void(const CameraFrame &)>;

// ─────────────────────────────────────────────────────────────────────────────
//  ISP 3A stubs (Auto White Balance, Auto Exposure, Auto Focus)
// ─────────────────────────────────────────────────────────────────────────────
struct Awb { float r_gain = 1.0f, g_gain = 1.0f, b_gain = 1.0f; };
struct Ae  { float ev = 0.0f; uint32_t iso = 100; uint32_t shutter_us = 16667; };
struct Af  { float focus_distance = 1.0f; bool locked = false; };

struct IspState { Awb awb; Ae ae; Af af; };

static void run_3a(IspState &state, const uint8_t * /*frame*/, size_t /*len*/) {
    // Real impl runs histogram analysis and calls into the ISP kernel driver.
    // Stub: no-op.
    (void)state;
}

// ─────────────────────────────────────────────────────────────────────────────
//  CameraPipeline
// ─────────────────────────────────────────────────────────────────────────────
class CameraPipeline {
public:
    CameraPipeline() = default;
    ~CameraPipeline() { close_device(); }

    bool open(const std::string &dev_path, uint32_t w = DEFAULT_W, uint32_t h = DEFAULT_H) {
        dev_path_ = dev_path;
        fd_ = ::open(dev_path.c_str(), O_RDWR | O_NONBLOCK);
        if (fd_ < 0) {
            fprintf(stderr, "[camera] open %s: %s\n", dev_path.c_str(), strerror(errno));
            return false;   // Non-fatal in emulator/CI builds
        }

        if (!set_format(w, h))    { ::close(fd_); fd_ = -1; return false; }
        if (!alloc_buffers())     { ::close(fd_); fd_ = -1; return false; }
        if (!start_streaming())   { ::close(fd_); fd_ = -1; return false; }

        width_ = w; height_ = h; streaming_ = true;
        fprintf(stderr, "[camera] opened %s %ux%u\n", dev_path.c_str(), w, h);
        return true;
    }

    bool open_stub(uint32_t w = DEFAULT_W, uint32_t h = DEFAULT_H) {
        width_ = w; height_ = h; streaming_ = true;
        stub_frame_.resize(w * h * 3 / 2, 128);  // NV12 gray
        fprintf(stderr, "[camera] opened stub device %ux%u\n", w, h);
        return true;
    }

    void close_device() {
        if (streaming_ && fd_ >= 0) stop_streaming();
        for (auto &b : bufs_)
            if (b.start && b.start != MAP_FAILED)
                ::munmap(b.start, b.length);
        bufs_.clear();
        if (fd_ >= 0) { ::close(fd_); fd_ = -1; }
        streaming_ = false;
    }

    // Call once per application frame to dequeue, process, and re-queue.
    bool capture_frame(FrameCallback cb) {
        if (!streaming_) return false;

        if (fd_ < 0) {
            // Stub path: synthesise a frame.
            ++sequence_;
            CameraFrame f{ stub_frame_.data(), stub_frame_.size(),
                           width_, height_, sequence_, 0 };
            run_3a(isp_, f.data, f.size);
            if (cb) cb(f);
            return true;
        }

        struct v4l2_buffer buf{};
        buf.type   = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        buf.memory = V4L2_MEMORY_MMAP;

        if (::ioctl(fd_, VIDIOC_DQBUF, &buf) < 0) return false;

        CameraFrame f{
            static_cast<const uint8_t *>(bufs_[buf.index].start),
            buf.bytesused, width_, height_, buf.sequence, 0
        };
        run_3a(isp_, f.data, f.size);
        if (cb) cb(f);

        // Re-queue the buffer.
        ::ioctl(fd_, VIDIOC_QBUF, &buf);
        return true;
    }

    bool is_open()   const { return streaming_; }
    uint32_t width()  const { return width_;  }
    uint32_t height() const { return height_; }
    const IspState &isp_state() const { return isp_; }

private:
    bool set_format(uint32_t w, uint32_t h) {
        struct v4l2_format fmt{};
        fmt.type                  = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        fmt.fmt.pix.width         = w;
        fmt.fmt.pix.height        = h;
        fmt.fmt.pix.pixelformat   = V4L2_PIX_FMT_NV12;
        fmt.fmt.pix.field         = 1; // V4L2_FIELD_NONE
        return ::ioctl(fd_, VIDIOC_S_FMT, &fmt) == 0;
    }

    bool alloc_buffers() {
        struct v4l2_requestbuffers req{};
        req.count  = NUM_BUFS;
        req.type   = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        req.memory = V4L2_MEMORY_MMAP;
        if (::ioctl(fd_, VIDIOC_REQBUFS, &req) < 0) return false;

        bufs_.resize(req.count);
        for (uint32_t i = 0; i < req.count; ++i) {
            struct v4l2_buffer buf{};
            buf.type   = V4L2_BUF_TYPE_VIDEO_CAPTURE;
            buf.memory = V4L2_MEMORY_MMAP;
            buf.index  = i;
            if (::ioctl(fd_, VIDIOC_QUERYBUF, &buf) < 0) return false;

            bufs_[i].length = buf.length;
            bufs_[i].start  = ::mmap(nullptr, buf.length,
                                      PROT_READ | PROT_WRITE, MAP_SHARED,
                                      fd_, buf.m.offset);
            if (bufs_[i].start == MAP_FAILED) return false;
            ::ioctl(fd_, VIDIOC_QBUF, &buf);
        }
        return true;
    }

    bool start_streaming() {
        int type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        return ::ioctl(fd_, VIDIOC_STREAMON, &type) == 0;
    }

    void stop_streaming() {
        int type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        ::ioctl(fd_, VIDIOC_STREAMOFF, &type);
    }

    int                    fd_        = -1;
    std::string            dev_path_;
    uint32_t               width_     = 0;
    uint32_t               height_    = 0;
    bool                   streaming_ = false;
    uint32_t               sequence_  = 0;
    std::vector<FrameBuffer> bufs_;
    std::vector<uint8_t>   stub_frame_;
    IspState               isp_;
};

} // namespace monoos::camera

extern "C" {
    static monoos::camera::CameraPipeline g_pipeline;

    int monoos_camera_open(const char *dev, uint32_t w, uint32_t h) {
        if (dev && dev[0])
            return g_pipeline.open(dev, w, h) ? 0 : -1;
        return g_pipeline.open_stub(w, h) ? 0 : -1;
    }

    void monoos_camera_close() { g_pipeline.close_device(); }

    int monoos_camera_capture(void (*cb)(const uint8_t *, uint32_t, uint32_t, uint32_t)) {
        return g_pipeline.capture_frame([cb](const monoos::camera::CameraFrame &f){
            if (cb) cb(f.data, f.size, f.width, f.height);
        }) ? 0 : -1;
    }
}
