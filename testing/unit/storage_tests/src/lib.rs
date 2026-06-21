//! Unit tests for the MonoOS storage / media store layer.

#[cfg(test)]
mod media_store_tests {
    use std::collections::HashMap;
    use std::path::Path;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum MediaType { Image, Video, Audio, Document, Other }

    fn media_type_of(ext: &str) -> MediaType {
        match ext.to_lowercase().as_str() {
            "jpg"|"jpeg"|"png"|"webp"|"heic" => MediaType::Image,
            "mp4"|"mkv"|"webm"|"mov"         => MediaType::Video,
            "mp3"|"flac"|"aac"|"opus"|"ogg"  => MediaType::Audio,
            "pdf"|"doc"|"docx"|"txt"         => MediaType::Document,
            _                                => MediaType::Other,
        }
    }

    struct MediaStore { entries: HashMap<u64, (String, MediaType)>, next: u64 }

    impl MediaStore {
        fn new() -> Self { MediaStore { entries: HashMap::new(), next: 1 } }
        fn index(&mut self, path: &Path) -> u64 {
            let ext   = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let mtype = media_type_of(ext);
            let id    = self.next; self.next += 1;
            self.entries.insert(id, (path.to_string_lossy().to_string(), mtype));
            id
        }
        fn remove(&mut self, id: u64) -> bool { self.entries.remove(&id).is_some() }
        fn count(&self) -> usize { self.entries.len() }
        fn by_type(&self, t: MediaType) -> usize {
            self.entries.values().filter(|(_, mt)| *mt == t).count()
        }
    }

    #[test]
    fn index_and_count() {
        let mut s = MediaStore::new();
        s.index(Path::new("/sdcard/photo.jpg"));
        s.index(Path::new("/sdcard/clip.mp4"));
        assert_eq!(s.count(), 2);
    }

    #[test]
    fn type_classification() {
        let mut s = MediaStore::new();
        s.index(Path::new("a.jpg"));
        s.index(Path::new("b.png"));
        s.index(Path::new("c.mp3"));
        assert_eq!(s.by_type(MediaType::Image), 2);
        assert_eq!(s.by_type(MediaType::Audio), 1);
    }

    #[test]
    fn remove_entry() {
        let mut s = MediaStore::new();
        let id = s.index(Path::new("test.jpg"));
        assert!(s.remove(id));
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn unknown_extension_is_other() {
        assert_eq!(media_type_of("xyz"), MediaType::Other);
    }
}

#[cfg(test)]
mod scoped_storage_tests {
    use std::path::PathBuf;

    struct ScopedStorage { data: PathBuf, cache: PathBuf, files: PathBuf }

    impl ScopedStorage {
        fn new(root: &str, pkg: &str) -> Self {
            let base = PathBuf::from(root).join(pkg);
            ScopedStorage { data: base.clone(), cache: base.join("cache"), files: base.join("files") }
        }
    }

    #[test]
    fn paths_contain_package_name() {
        let s = ScopedStorage::new("/data/data", "com.example.app");
        assert!(s.data.to_str().unwrap().contains("com.example.app"));
        assert!(s.cache.to_str().unwrap().ends_with("cache"));
        assert!(s.files.to_str().unwrap().ends_with("files"));
    }

    #[test]
    fn different_packages_isolated() {
        let a = ScopedStorage::new("/data/data", "com.app.a");
        let b = ScopedStorage::new("/data/data", "com.app.b");
        assert_ne!(a.data, b.data);
    }
}
