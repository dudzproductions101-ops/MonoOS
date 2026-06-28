//! Integration tests for monoos-sdk, exercised against the mock runtime.
//! Run with: cargo test --features mock-runtime

#![cfg(feature = "mock-runtime")]

use monoos_sdk::context::Context;
use monoos_sdk::permissions::{self, GrantState, Permission};
use monoos_sdk::{mock_runtime, network, notifications, storage, ui};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn context_create_and_package_name() {
    let ctx = Context::create("com.monoos.test", 1).expect("context should be created");
    assert_eq!(ctx.package_name(), "com.monoos.mock");
}

#[test]
fn context_create_rejects_nul_byte() {
    assert!(Context::create("com.monoos.\0bad", 1).is_none());
}

#[test]
fn permission_starts_not_requested() {
    mock_runtime::mock_reset();
    assert_eq!(permissions::permission_state(Permission::Camera), GrantState::NotRequested);
    assert!(permissions::check_permission(Permission::Camera).is_err());
}

#[test]
fn permission_request_grants_and_updates_state() {
    mock_runtime::mock_reset();
    let result = Rc::new(Cell::new(None));
    let result2 = result.clone();
    permissions::request_permission(Permission::Microphone, move |granted| {
        result2.set(Some(granted));
    });
    assert_eq!(result.get(), Some(true));
    assert_eq!(permissions::permission_state(Permission::Microphone), GrantState::Granted);
    assert!(permissions::check_permission(Permission::Microphone).is_ok());
}

#[test]
fn request_permissions_invokes_callback_for_each() {
    mock_runtime::mock_reset();
    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let seen2 = seen.clone();
    permissions::request_permissions(
        &[Permission::Camera, Permission::Location, Permission::Storage],
        move |p, granted| {
            seen2.borrow_mut().push((p, granted));
        },
    );
    let seen = seen.borrow();
    assert_eq!(seen.len(), 3);
    assert!(seen.iter().all(|(_, g)| *g));
}

#[test]
fn storage_dirs_are_non_empty() {
    assert!(!storage::files_dir().as_os_str().is_empty());
    assert!(!storage::cache_dir().as_os_str().is_empty());
    assert!(!storage::db_dir().as_os_str().is_empty());
}

#[test]
fn query_media_succeeds_with_empty_mock_store() {
    let entries = storage::query_media(storage::MediaType::Image).expect("query should succeed");
    assert!(entries.is_empty());
}

#[test]
fn insert_media_returns_uri() {
    let path = std::path::Path::new("/tmp/test.jpg");
    let result = storage::insert_media(path, "image/jpeg");
    assert!(result.is_ok());
}

#[test]
fn notification_channel_create_and_post() {
    let channel = notifications::Channel {
        id: "default".into(),
        name: "Default".into(),
        description: "General notifications".into(),
        importance: notifications::Priority::Default,
        vibrate: true,
        show_badge: true,
    };
    assert!(channel.create().is_ok());

    let notif = notifications::Notification {
        id: 1,
        channel_id: "default".into(),
        title: "Hello".into(),
        body: "World".into(),
        ticker: "Hello: World".into(),
        priority: notifications::Priority::Default,
        auto_cancel: true,
        ongoing: false,
        badge_count: 1,
    };
    assert!(notif.post().is_ok());
    assert!(notifications::cancel(1).is_ok());
    notifications::cancel_all();
}

#[test]
fn network_get_state_reports_connected_wifi_in_mock() {
    let state = network::get_state().expect("should succeed");
    assert!(state.connected);
    assert_eq!(state.net_type, network::NetworkType::Wifi);
}

#[test]
fn network_resolve_calls_back_with_address() {
    let result = Rc::new(Cell::new(false));
    let result2 = result.clone();
    network::resolve("example.com", move |r| {
        result2.set(r.is_ok());
    })
    .expect("resolve call should be accepted");
    assert!(result.get());
}

#[test]
fn ui_load_qml_accepts_valid_path() {
    assert!(ui::load_qml("res/qml/Main.qml").is_ok());
}

#[test]
fn ui_load_qml_rejects_nul_byte() {
    assert!(ui::load_qml("res/\0bad.qml").is_err());
}

#[test]
fn audio_stream_open_write_close() {
    use monoos_sdk::audio::{AudioStream, AudioUsage};
    let mut stream = AudioStream::open(48_000, 2, AudioUsage::Media, 0.8).expect("stream should open");
    let silence = vec![0.0f32; 960]; // 480 stereo frames
    let accepted = stream.write(&silence).expect("write should succeed");
    assert_eq!(accepted, silence.len() as u32);
    assert!(stream.set_volume(0.5).is_ok());
}

#[test]
fn audio_set_master_volume_clamps_and_succeeds() {
    use monoos_sdk::audio;
    assert!(audio::set_master_volume(1.5).is_ok()); // clamped internally to 1.0
}

#[test]
fn player_lifecycle_transitions_state() {
    use monoos_sdk::media::{Player, PlayerState};
    let mut p = Player::new();
    assert_eq!(p.state(), PlayerState::Idle);
    p.set_uri("file:///music/track.mp3").expect("set_uri should succeed");
    p.prepare().expect("prepare should succeed");
    assert_eq!(p.state(), PlayerState::Prepared);
    p.start().expect("start should succeed");
    assert_eq!(p.state(), PlayerState::Started);
    p.pause().expect("pause should succeed");
    assert_eq!(p.state(), PlayerState::Paused);
    p.stop().expect("stop should succeed");
    assert_eq!(p.state(), PlayerState::Stopped);
    assert_eq!(p.position(), 0);
}

#[test]
fn player_seek_updates_position() {
    use monoos_sdk::media::Player;
    let mut p = Player::new();
    p.seek(42_000).expect("seek should succeed");
    assert_eq!(p.position(), 42_000);
}

#[test]
fn player_listener_receives_state_callback() {
    use monoos_sdk::media::{Player, PlayerListener, PlayerState};
    let seen = Rc::new(Cell::new(None));
    let seen2 = seen.clone();
    let mut p = Player::new();
    p.set_listener(PlayerListener {
        on_state: Some(Box::new(move |s| seen2.set(Some(s)))),
        ..Default::default()
    });
    assert_eq!(seen.get(), Some(PlayerState::Idle));
}

#[test]
fn media_store_query_insert_delete() {
    use monoos_sdk::storage::{MediaStore, MediaType};
    let store = MediaStore::new();
    let entries = store.query(MediaType::Audio).expect("query should succeed");
    assert!(entries.is_empty());
    let uri = store
        .insert(std::path::Path::new("/tmp/song.mp3"), "audio/mpeg")
        .expect("insert should succeed");
    assert!(store.delete(&uri).is_ok());
}
