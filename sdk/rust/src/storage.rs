//! storage.rs – Safe wrapper around `monoos_storage.h`.

use crate::result::{check, MonoOsResult};
use crate::sys;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::path::PathBuf;

/// The kind of shared media a [`MediaEntry`] represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Document,
    Other,
}

impl From<sys::MonoOS_MediaType> for MediaType {
    fn from(t: sys::MonoOS_MediaType) -> Self {
        match t {
            sys::MonoOS_MediaType::Image => MediaType::Image,
            sys::MonoOS_MediaType::Video => MediaType::Video,
            sys::MonoOS_MediaType::Audio => MediaType::Audio,
            sys::MonoOS_MediaType::Document => MediaType::Document,
            sys::MonoOS_MediaType::Other => MediaType::Other,
        }
    }
}

impl From<MediaType> for sys::MonoOS_MediaType {
    fn from(t: MediaType) -> Self {
        match t {
            MediaType::Image => sys::MonoOS_MediaType::Image,
            MediaType::Video => sys::MonoOS_MediaType::Video,
            MediaType::Audio => sys::MonoOS_MediaType::Audio,
            MediaType::Document => sys::MonoOS_MediaType::Document,
            MediaType::Other => sys::MonoOS_MediaType::Other,
        }
    }
}

/// An opaque content URI identifying a file in the shared media store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentUri(pub String);

/// A single entry returned from [`query_media`].
#[derive(Debug, Clone)]
pub struct MediaEntry {
    pub uri: ContentUri,
    pub display_name: String,
    pub size_bytes: u64,
    pub media_type: MediaType,
    pub mime_type: String,
    pub date_added: u64,
    pub date_modified: u64,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
}

fn cstr_array_to_string(buf: &[std::os::raw::c_char]) -> String {
    // SAFETY: buf is a fixed-size, NUL-terminated (or fully-populated)
    // char array coming from the C ABI; find the NUL ourselves to avoid
    // reading past a non-terminated buffer.
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len()) };
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

impl From<&sys::MonoOS_MediaEntry> for MediaEntry {
    fn from(e: &sys::MonoOS_MediaEntry) -> Self {
        MediaEntry {
            uri: ContentUri(cstr_array_to_string(&e.uri.uri)),
            display_name: cstr_array_to_string(&e.display_name),
            size_bytes: e.size_bytes,
            media_type: e.media_type.into(),
            mime_type: cstr_array_to_string(&e.mime_type),
            date_added: e.date_added,
            date_modified: e.date_modified,
            width: e.width,
            height: e.height,
            duration_ms: e.duration_ms,
        }
    }
}

/// The app's private files directory (created automatically on first access).
pub fn files_dir() -> PathBuf {
    PathBuf::from(unsafe { cstr_to_string(sys::monoos_files_dir()) })
}

/// The app's private cache directory.
pub fn cache_dir() -> PathBuf {
    PathBuf::from(unsafe { cstr_to_string(sys::monoos_cache_dir()) })
}

/// The app's private database directory.
pub fn db_dir() -> PathBuf {
    PathBuf::from(unsafe { cstr_to_string(sys::monoos_db_dir()) })
}

unsafe fn cstr_to_string(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Object-oriented convenience wrapper over the free functions in this
/// module ([`query_media`], [`insert_media`], [`delete_media`]). Holds no
/// state of its own — equivalent to calling the free functions directly.
#[derive(Debug, Default, Clone, Copy)]
pub struct MediaStore;

impl MediaStore {
    pub fn new() -> Self {
        MediaStore
    }

    pub fn query(&self, media_type: MediaType) -> MonoOsResult<Vec<MediaEntry>> {
        query_media(media_type)
    }

    pub fn insert(&self, path: &std::path::Path, mime_type: &str) -> MonoOsResult<ContentUri> {
        insert_media(path, mime_type)
    }

    pub fn delete(&self, uri: &ContentUri) -> MonoOsResult<()> {
        delete_media(uri)
    }
}

/// Query the shared media store for all entries of a given type. Requires
/// [`crate::permissions::Permission::Storage`].
pub fn query_media(media_type: MediaType) -> MonoOsResult<Vec<MediaEntry>> {
    let mut results: Vec<MediaEntry> = Vec::new();
    let code = unsafe {
        sys::monoos_media_query(
            media_type.into(),
            collect_trampoline,
            &mut results as *mut Vec<MediaEntry> as *mut c_void,
        )
    };
    check(code)?;
    Ok(results)
}

extern "C" fn collect_trampoline(entry: *const sys::MonoOS_MediaEntry, user_data: *mut c_void) {
    if entry.is_null() || user_data.is_null() {
        return;
    }
    let results: &mut Vec<MediaEntry> = unsafe { &mut *(user_data as *mut Vec<MediaEntry>) };
    results.push(unsafe { &*entry }.into());
}

/// Insert a file into the shared media store, returning its assigned
/// content URI. Requires [`crate::permissions::Permission::Storage`].
pub fn insert_media(path: &std::path::Path, mime_type: &str) -> MonoOsResult<ContentUri> {
    let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| crate::result::MonoOsError::InvalidArg)?;
    let c_mime = CString::new(mime_type).map_err(|_| crate::result::MonoOsError::InvalidArg)?;
    let mut out = sys::MonoOS_ContentUri { uri: [0; 256] };
    let code = unsafe { sys::monoos_media_insert(c_path.as_ptr(), c_mime.as_ptr(), &mut out) };
    check(code)?;
    Ok(ContentUri(cstr_array_to_string(&out.uri)))
}

/// Delete an entry from the shared media store. The caller must own the
/// file or hold the manage-media permission.
pub fn delete_media(uri: &ContentUri) -> MonoOsResult<()> {
    let mut raw = sys::MonoOS_ContentUri { uri: [0; 256] };
    let bytes = uri.0.as_bytes();
    let n = bytes.len().min(255);
    for (i, b) in bytes[..n].iter().enumerate() {
        raw.uri[i] = *b as std::os::raw::c_char;
    }
    check(unsafe { sys::monoos_media_delete(&raw) })
}
