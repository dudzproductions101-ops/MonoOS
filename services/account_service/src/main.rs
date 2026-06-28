//! account_service — stub binary
fn main() {
    eprintln!("[account_service] starting (pid={})", std::process::id());
    loop { std::thread::sleep(std::time::Duration::from_secs(60)); }
}
