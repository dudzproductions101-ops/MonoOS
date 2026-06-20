//! storage_manager.rs – OneOS Framework Storage Manager
//!
//! Provides the app-facing API for file system access:
//!   - Scoped storage: apps access only their own directory by default.
//!   - Media store: index-based access to shared photos/videos/audio.
//!   - Document picker: present a system UI to let the user choose a file.
//!   - External storage: mount/unmount and query removable media.

use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// A content URI that uniquely identifies a file in the media store.
/// Format: "content://media/<type>/<id>"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentUri(String);

impl ContentUri {
    pub fn new(uri: impl Into<String>) -> Self { ContentUri(uri.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Media type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType { Image, Video, Audio, Document, Other }

impl MediaType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpg"|"jpeg"|"png"|"webp"|"heic"|"gif" => MediaType::Image,
            "mp4"|"mkv"|"webm"|"mov"|"avi"         => MediaType::Video,
            "mp3"|"flac"|"aac"|"opus"|"ogg"|"wav"  => MediaType::Audio,
            "pdf"|"doc"|"docx"|"txt"|"odt"         => MediaType::Document,
            _                                       => MediaType::Other,
        }
    }
}

/// A single media store entry.
#[derive(Debug, Clone)]
pub struct MediaEntry {
    pub uri:          ContentUri,
    pub display_name: String,
    pub size_bytes:   u64,
    pub media_type:   MediaType,
    pub mime_type:    String,
    pub date_added:   u64,   // Unix seconds
    pub date_modified: u64,
    pub width:        Option<u32>,
    pub height:       Option<u32>,
    pub duration_ms:  Option<u64>,
    pub is_favourite: bool,
}

/// Scoped storage: per-app directory access.
pub struct ScopedStorage {
    app_data_dir:  PathBuf,
    app_cache_dir: PathBuf,
    app_files_dir: PathBuf,
}

impl ScopedStorage {
    pub fn new(data_root: &Path, package: &str) -> Self {
        let base = data_root.join(package);
        ScopedStorage {
            app_data_dir:  base.clone(),
            app_cache_dir: base.join("cache"),
            app_files_dir: base.join("files"),
        }
    }

    pub fn data_dir(&self)  -> &Path { &self.app_data_dir  }
    pub fn cache_dir(&self) -> &Path { &self.app_cache_dir }
    pub fn files_dir(&self) -> &Path { &self.app_files_dir }

    /// Return the path for a named file in the app's files dir.
    pub fn file_path(&self, name: &str) -> PathBuf {
        self.app_files_dir.join(name)
    }

    /// Ensure all app directories exist.
    pub fn create_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.app_data_dir)?;
        std::fs::create_dir_all(&self.app_cache_dir)?;
        std::fs::create_dir_all(&self.app_files_dir)
    }
}

/// The media store index — a lightweight in-memory index backed by SQLite.
/// (SQLite interaction is through a C FFI shim in the real build.)
pub struct MediaStore {
    entries: HashMap<ContentUri, MediaEntry>,
    next_id: u64,
}

impl MediaStore {
    pub fn new() -> Self { MediaStore { entries: HashMap::new(), next_id: 1 } }

    /// Index a new file.  Returns its ContentUri.
    pub fn index(&mut self, path: &Path) -> Option<ContentUri> {
        let name = path.file_name()?.to_str()?.to_owned();
        let ext  = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let mtype = MediaType::from_extension(ext);
        let type_str = match mtype {
            MediaType::Image    => "images",
            MediaType::Video    => "video",
            MediaType::Audio    => "audio",
            MediaType::Document => "documents",
            MediaType::Other    => "files",
        };
        let id  = self.next_id;
        self.next_id += 1;
        let uri = ContentUri::new(format!("content://media/{type_str}/{id}"));
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        self.entries.insert(uri.clone(), MediaEntry {
            uri:           uri.clone(),
            display_name:  name,
            size_bytes:    size,
            media_type:    mtype,
            mime_type:     format!("{}/{}", type_str, ext),
            date_added:    0,
            date_modified: 0,
            width: None, height: None, duration_ms: None,
            is_favourite: false,
        });
        Some(uri)
    }

    pub fn get(&self, uri: &ContentUri) -> Option<&MediaEntry> {
        self.entries.get(uri)
    }

    pub fn remove(&mut self, uri: &ContentUri) -> bool {
        self.entries.remove(uri).is_some()
    }

    pub fn query_by_type(&self, mtype: MediaType) -> Vec<&MediaEntry> {
        self.entries.values().filter(|e| e.media_type == mtype).collect()
    }

    pub fn count(&self) -> usize { self.entries.len() }
}

impl Default for MediaStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_type_from_ext() {
        assert_eq!(MediaType::from_extension("jpg"),  MediaType::Image);
        assert_eq!(MediaType::from_extension("mp4"),  MediaType::Video);
        assert_eq!(MediaType::from_extension("mp3"),  MediaType::Audio);
        assert_eq!(MediaType::from_extension("pdf"),  MediaType::Document);
        assert_eq!(MediaType::from_extension("xyz"),  MediaType::Other);
    }

    #[test]
    fn media_store_index_and_query() {
        let mut store = MediaStore::new();
        let tmp = std::env::temp_dir().join("test_photo.jpg");
        std::fs::write(&tmp, b"fake jpeg").ok();
        let uri = store.index(&tmp).expect("index failed");
        assert!(store.get(&uri).is_some());
        let images = store.query_by_type(MediaType::Image);
        assert_eq!(images.len(), 1);
        assert!(store.remove(&uri));
        assert_eq!(store.count(), 0);
    }
}
