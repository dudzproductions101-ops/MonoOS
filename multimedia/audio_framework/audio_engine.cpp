// audio_engine.cpp – MonoOS Audio Engine
//
// Central audio routing graph for MonoOS.  Built on top of ALSA (via libasound)
// with an abstraction layer that lets the audio HAL plug in vendor-specific
// DSP pipelines.
//
// Architecture:
//   AudioStream  → AudioMixer  → AudioEffect chain  → AudioOutputDevice
//
// Each application opens an AudioStream.  The AudioMixer combines all active
// streams (with software volume and per-stream sample-rate conversion) and
// feeds the result to the selected AudioOutputDevice (speaker, headset,
// Bluetooth A2DP, HDMI).

#include <algorithm>
#include <array>
#include <atomic>
#include <cassert>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

namespace monoos::audio {

// ─────────────────────────────────────────────────────────────────────────────
//  Constants
// ─────────────────────────────────────────────────────────────────────────────
static constexpr uint32_t SAMPLE_RATE    = 48000;
static constexpr uint32_t CHANNELS       = 2;
static constexpr uint32_t FRAMES_PER_BUF = 256;     // ~5.3 ms latency
static constexpr float    MAX_VOLUME     = 1.0f;

// ─────────────────────────────────────────────────────────────────────────────
//  AudioUsage – why the stream exists (affects routing and ducking)
// ─────────────────────────────────────────────────────────────────────────────
enum class AudioUsage {
    Media,       // music, podcasts
    Notification,
    Ringtone,
    VoiceCall,
    Alarm,
    Accessibility,
    Assistant,
    Game,
};

// ─────────────────────────────────────────────────────────────────────────────
//  AudioFormat
// ─────────────────────────────────────────────────────────────────────────────
enum class AudioFormat { PCM_16, PCM_32, FLOAT32 };

static size_t bytes_per_sample(AudioFormat fmt) {
    switch (fmt) {
        case AudioFormat::PCM_16:  return 2;
        case AudioFormat::PCM_32:  return 4;
        case AudioFormat::FLOAT32: return 4;
    }
    return 2;
}

// ─────────────────────────────────────────────────────────────────────────────
//  AudioStreamConfig
// ─────────────────────────────────────────────────────────────────────────────
struct AudioStreamConfig {
    uint32_t    sample_rate  = SAMPLE_RATE;
    uint32_t    channels     = CHANNELS;
    AudioFormat format       = AudioFormat::FLOAT32;
    AudioUsage  usage        = AudioUsage::Media;
    float       volume       = 1.0f;
    uint32_t    uid          = 0;        // owning app uid
};

// ─────────────────────────────────────────────────────────────────────────────
//  AudioStream – one per app audio session
// ─────────────────────────────────────────────────────────────────────────────
class AudioStream {
public:
    explicit AudioStream(uint32_t id, AudioStreamConfig cfg)
        : id_(id), cfg_(cfg), active_(false), ducked_(false) {
        buf_.resize(FRAMES_PER_BUF * cfg_.channels, 0.0f);
    }

    uint32_t id() const { return id_; }
    const AudioStreamConfig &config() const { return cfg_; }
    bool is_active() const { return active_.load(std::memory_order_relaxed); }
    void set_active(bool v) { active_.store(v, std::memory_order_relaxed); }

    void set_volume(float v) { cfg_.volume = std::clamp(v, 0.0f, MAX_VOLUME); }
    float effective_volume() const { return ducked_ ? cfg_.volume * 0.2f : cfg_.volume; }

    void duck()   { ducked_ = true; }
    void unduck() { ducked_ = false; }

    // Fill internal buffer with silence (apps write via write_pcm).
    void write_pcm(const float *src, size_t frames) {
        std::lock_guard<std::mutex> lk(mu_);
        size_t samples = std::min(frames * cfg_.channels, buf_.size());
        std::memcpy(buf_.data(), src, samples * sizeof(float));
    }

    // Mix this stream's buffer into dst, applying volume.
    void mix_into(float *dst, size_t frames) const {
        std::lock_guard<std::mutex> lk(mu_);
        if (!active_.load()) return;
        float vol = effective_volume();
        for (size_t i = 0; i < frames * CHANNELS && i < buf_.size(); ++i)
            dst[i] += buf_[i] * vol;
    }

private:
    uint32_t              id_;
    AudioStreamConfig     cfg_;
    std::atomic<bool>     active_;
    bool                  ducked_;
    std::vector<float>    buf_;
    mutable std::mutex    mu_;
};

// ─────────────────────────────────────────────────────────────────────────────
//  AudioFocusState – which stream "owns" the audio channel
// ─────────────────────────────────────────────────────────────────────────────
enum class FocusResult { Granted, Denied, WaitForGain };

// ─────────────────────────────────────────────────────────────────────────────
//  AudioMixer – combines all active streams
// ─────────────────────────────────────────────────────────────────────────────
class AudioMixer {
public:
    AudioMixer() : master_volume_(1.0f) {
        mix_buf_.resize(FRAMES_PER_BUF * CHANNELS, 0.0f);
    }

    void add_stream(std::shared_ptr<AudioStream> s) {
        std::lock_guard<std::mutex> lk(mu_);
        streams_.push_back(std::move(s));
    }

    void remove_stream(uint32_t id) {
        std::lock_guard<std::mutex> lk(mu_);
        streams_.erase(std::remove_if(streams_.begin(), streams_.end(),
            [id](const auto &s){ return s->id() == id; }), streams_.end());
    }

    // Mix all streams → mix_buf_ and return pointer to it.
    const float *mix(size_t frames) {
        std::lock_guard<std::mutex> lk(mu_);
        std::fill(mix_buf_.begin(), mix_buf_.end(), 0.0f);
        for (auto &s : streams_)
            s->mix_into(mix_buf_.data(), frames);
        // Apply master volume and soft-clip.
        for (float &s : mix_buf_)
            s = std::tanh(s * master_volume_);
        return mix_buf_.data();
    }

    void set_master_volume(float v) { master_volume_ = std::clamp(v, 0.0f, 1.0f); }
    float master_volume() const { return master_volume_; }

    size_t stream_count() const {
        std::lock_guard<std::mutex> lk(mu_);
        return streams_.size();
    }

private:
    std::vector<std::shared_ptr<AudioStream>> streams_;
    std::vector<float>                        mix_buf_;
    float                                     master_volume_;
    mutable std::mutex                        mu_;
};

// ─────────────────────────────────────────────────────────────────────────────
//  AudioEngine – singleton orchestrator
// ─────────────────────────────────────────────────────────────────────────────
class AudioEngine {
public:
    static AudioEngine &instance() {
        static AudioEngine eng;
        return eng;
    }

    bool init() {
        mixer_ = std::make_unique<AudioMixer>();
        running_.store(true);
        io_thread_ = std::thread(&AudioEngine::io_loop, this);
        fprintf(stderr, "[audio_engine] init OK (%u Hz, %u ch, %u frames)\n",
                SAMPLE_RATE, CHANNELS, FRAMES_PER_BUF);
        return true;
    }

    void shutdown() {
        running_.store(false);
        if (io_thread_.joinable()) io_thread_.join();
        fprintf(stderr, "[audio_engine] shutdown, total_frames=%llu\n",
                (unsigned long long)total_frames_written_.load());
    }

    uint32_t open_stream(const AudioStreamConfig &cfg) {
        std::lock_guard<std::mutex> lk(mu_);
        uint32_t id = next_id_++;
        auto s = std::make_shared<AudioStream>(id, cfg);
        s->set_active(true);
        mixer_->add_stream(s);
        streams_by_id_[id] = s;
        request_focus(id, cfg.usage);
        return id;
    }

    void close_stream(uint32_t id) {
        std::lock_guard<std::mutex> lk(mu_);
        mixer_->remove_stream(id);
        streams_by_id_.erase(id);
        if (focus_holder_ == id) focus_holder_ = 0;
    }

    void write_pcm(uint32_t id, const float *data, size_t frames) {
        std::lock_guard<std::mutex> lk(mu_);
        auto it = streams_by_id_.find(id);
        if (it != streams_by_id_.end())
            it->second->write_pcm(data, frames);
    }

    void set_stream_volume(uint32_t id, float v) {
        std::lock_guard<std::mutex> lk(mu_);
        auto it = streams_by_id_.find(id);
        if (it != streams_by_id_.end())
            it->second->set_volume(v);
    }

    void set_master_volume(float v) {
        if (mixer_) mixer_->set_master_volume(v);
    }

    size_t active_stream_count() const { return mixer_ ? mixer_->stream_count() : 0; }

private:
    AudioEngine() = default;

    FocusResult request_focus(uint32_t id, AudioUsage usage) {
        // Ringtone / calls always preempt; duck others.
        if (usage == AudioUsage::VoiceCall || usage == AudioUsage::Ringtone) {
            for (auto &[sid, s] : streams_by_id_)
                if (sid != id) s->duck();
        } else if (focus_holder_ != 0) {
            auto it = streams_by_id_.find(focus_holder_);
            if (it != streams_by_id_.end())
                it->second->duck();
        }
        focus_holder_ = id;
        return FocusResult::Granted;
    }

    void io_loop() {
        // In a real driver this opens an ALSA PCM device and calls
        // snd_pcm_writei() in a real-time loop.  Here we simulate the
        // timing without actual hardware I/O.
        using Clock = std::chrono::steady_clock;
        const auto period = std::chrono::microseconds(
            static_cast<long>(1'000'000.0 * FRAMES_PER_BUF / SAMPLE_RATE));

        while (running_.load()) {
            auto deadline = Clock::now() + period;
            if (mixer_) {
                mixer_->mix(FRAMES_PER_BUF);
                total_frames_written_ += FRAMES_PER_BUF;
            }
            std::this_thread::sleep_until(deadline);
        }
    }

    std::unique_ptr<AudioMixer>                              mixer_;
    std::unordered_map<uint32_t, std::shared_ptr<AudioStream>> streams_by_id_;
    uint32_t                                                  next_id_      = 1;
    uint32_t                                                  focus_holder_ = 0;
    std::atomic<bool>                                         running_{false};
    std::atomic<uint64_t>                                     total_frames_written_{0};
    std::thread                                               io_thread_;
    mutable std::mutex                                        mu_;
};

} // namespace monoos::audio

// ─────────────────────────────────────────────────────────────────────────────
//  C API consumed by the audio_service (Rust FFI)
// ─────────────────────────────────────────────────────────────────────────────
extern "C" {
    int      monoos_audio_init()                              { return monoos::audio::AudioEngine::instance().init() ? 0 : -1; }
    void     monoos_audio_shutdown()                          { monoos::audio::AudioEngine::instance().shutdown(); }
    uint32_t monoos_audio_open_stream(uint32_t uid, int usage, float vol) {
        monoos::audio::AudioStreamConfig cfg;
        cfg.uid    = uid;
        cfg.usage  = static_cast<monoos::audio::AudioUsage>(usage);
        cfg.volume = vol;
        return monoos::audio::AudioEngine::instance().open_stream(cfg);
    }
    void monoos_audio_close_stream(uint32_t id)               { monoos::audio::AudioEngine::instance().close_stream(id); }
    void monoos_audio_write(uint32_t id, const float *d, uint32_t frames) { monoos::audio::AudioEngine::instance().write_pcm(id, d, frames); }
    void monoos_audio_set_volume(uint32_t id, float v)        { monoos::audio::AudioEngine::instance().set_stream_volume(id, v); }
    void monoos_audio_set_master(float v)                     { monoos::audio::AudioEngine::instance().set_master_volume(v); }
}
