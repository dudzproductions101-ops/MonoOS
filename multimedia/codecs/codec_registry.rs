//! codec_registry.rs – MonoOS Codec Registry
//!
//! Enumerates all hardware and software codecs available on the device
//! and provides codec selection for encode/decode operations.
//! Hardware codecs are discovered via the V4L2 M2M interface; software
//! codecs use FFmpeg or Rust-native implementations.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind { Video, Audio, Image }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodecOp { Encode, Decode }

#[derive(Debug, Clone)]
pub struct CodecInfo {
    pub name:         String,
    pub mime_type:    String,
    pub kind:         MediaKind,
    pub op:           CodecOp,
    pub hardware:     bool,
    /// V4L2 M2M device path for hardware codecs (e.g. "/dev/video1").
    pub device_path:  Option<String>,
    pub max_width:    u32,
    pub max_height:   u32,
    pub max_fps:      u32,
    pub bit_depth:    u8,
}

impl CodecInfo {
    pub fn software(name: &str, mime: &str, kind: MediaKind, op: CodecOp) -> Self {
        CodecInfo {
            name: name.into(), mime_type: mime.into(), kind, op,
            hardware: false, device_path: None,
            max_width: 7680, max_height: 4320, max_fps: 240, bit_depth: 10,
        }
    }

    pub fn hardware(name: &str, mime: &str, kind: MediaKind, op: CodecOp, dev: &str) -> Self {
        CodecInfo {
            name: name.into(), mime_type: mime.into(), kind, op,
            hardware: true, device_path: Some(dev.into()),
            max_width: 3840, max_height: 2160, max_fps: 60, bit_depth: 10,
        }
    }
}

/// The global codec registry, populated at startup.
pub struct CodecRegistry {
    codecs: Vec<CodecInfo>,
}

impl CodecRegistry {
    pub fn new() -> Self {
        let mut r = CodecRegistry { codecs: Vec::new() };
        r.register_builtins();
        r
    }

    fn register_builtins(&mut self) {
        use MediaKind::*; use CodecOp::*;

        // ── Software codecs ──────────────────────────────────────────────────
        let sw = [
            ("H.264/AVC SW Dec",  "video/avc",       Video, Decode),
            ("H.265/HEVC SW Dec", "video/hevc",       Video, Decode),
            ("VP9 SW Dec",        "video/x-vnd.on2.vp9", Video, Decode),
            ("AV1 SW Dec",        "video/av01",       Video, Decode),
            ("H.264/AVC SW Enc",  "video/avc",        Video, Encode),
            ("H.265/HEVC SW Enc", "video/hevc",       Video, Encode),
            ("AAC SW Dec",        "audio/mp4a-latm",  Audio, Decode),
            ("AAC SW Enc",        "audio/mp4a-latm",  Audio, Encode),
            ("Opus SW Dec",       "audio/opus",       Audio, Decode),
            ("Opus SW Enc",       "audio/opus",       Audio, Encode),
            ("FLAC SW Dec",       "audio/flac",       Audio, Decode),
            ("JPEG SW Dec",       "image/jpeg",       Image, Decode),
            ("JPEG SW Enc",       "image/jpeg",       Image, Encode),
            ("WebP SW Dec",       "image/webp",       Image, Decode),
            ("HEIC SW Dec",       "image/heic",       Image, Decode),
        ];

        for (name, mime, kind, op) in sw {
            self.codecs.push(CodecInfo::software(name, mime, kind, op));
        }

        // ── Hardware codecs (V4L2 M2M — device paths are SoC-specific) ──────
        let hw = [
            ("H.264 HW Dec", "video/avc",  Video, Decode, "/dev/video10"),
            ("H.264 HW Enc", "video/avc",  Video, Encode, "/dev/video11"),
            ("H.265 HW Dec", "video/hevc", Video, Decode, "/dev/video12"),
            ("H.265 HW Enc", "video/hevc", Video, Encode, "/dev/video13"),
        ];

        for (name, mime, kind, op, dev) in hw {
            self.codecs.push(CodecInfo::hardware(name, mime, kind, op, dev));
        }
    }

    /// Find the best codec for the given MIME type and operation.
    /// Prefers hardware over software.
    pub fn find(&self, mime: &str, op: CodecOp) -> Option<&CodecInfo> {
        // Hardware-accelerated first.
        self.codecs.iter()
            .filter(|c| c.mime_type == mime && c.op == op && c.hardware)
            .next()
            .or_else(|| {
                self.codecs.iter()
                    .find(|c| c.mime_type == mime && c.op == op)
            })
    }

    pub fn find_all(&self, mime: &str, op: CodecOp) -> Vec<&CodecInfo> {
        self.codecs.iter()
            .filter(|c| c.mime_type == mime && c.op == op)
            .collect()
    }

    pub fn list_by_kind(&self, kind: MediaKind) -> Vec<&CodecInfo> {
        self.codecs.iter().filter(|c| c.kind == kind).collect()
    }

    pub fn codec_count(&self) -> usize { self.codecs.len() }

    /// Returns true if the given MIME type can be decoded.
    pub fn can_decode(&self, mime: &str) -> bool {
        self.find(mime, CodecOp::Decode).is_some()
    }

    /// Returns true if the given MIME type can be encoded.
    pub fn can_encode(&self, mime: &str) -> bool {
        self.find(mime, CodecOp::Encode).is_some()
    }
}

impl Default for CodecRegistry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_decode_avc() {
        let r = CodecRegistry::new();
        assert!(r.can_decode("video/avc"));
    }

    #[test]
    fn prefers_hardware() {
        let r = CodecRegistry::new();
        let c = r.find("video/avc", CodecOp::Decode).unwrap();
        assert!(c.hardware);
    }

    #[test]
    fn can_encode_opus() {
        let r = CodecRegistry::new();
        assert!(r.can_encode("audio/opus"));
    }

    #[test]
    fn jpeg_decode_available() {
        let r = CodecRegistry::new();
        assert!(r.can_decode("image/jpeg"));
    }
}
