//! bluetooth_service — stub binary
fn main() {
    eprintln!("[bluetooth_service] starting (pid={})", std::process::id());
    loop { std::thread::sleep(std::time::Duration::from_secs(60)); }
}
