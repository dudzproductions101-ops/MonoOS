//! kernel_bridge.rs – MonoOS userspace ↔ kernel permission bridge
//!
//! Opens `/dev/monoos` and issues ioctl calls to grant, revoke, or query
//! permissions in the kernel's live process table.  The kernel side is
//! `kernel/core/syscalls/syscalls.c`; the ioctl numbers and `PermReq` layout
//! must match exactly.
//!
//! # Usage
//!
//! ```rust,no_run
//! use framework::permissions::kernel_bridge::KernelBridge;
//! use framework::permissions::permission_manager::Permission;
//!
//! let bridge = KernelBridge::open().expect("cannot open /dev/monoos");
//! bridge.grant(1234, Permission::Camera.kernel_bit()).expect("grant failed");
//! ```
//!
//! The bridge is `Send + Sync` (the underlying `File` is just an fd) so it
//! can be held inside an `Arc` across threads.

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

// ─────────────────────────────────────────────────────────────────────────────
//  ioctl numbers  (must match kernel/core/syscalls/syscalls.c)
//
//  MONOOS_IOC_MAGIC    = 'm'   (0x6d)
//  MONOOS_IOC_PERM_CHECK = _IOR('m', 1, struct monoos_perm_req)
//                        direction=READ(2) size=sizeof(PermReq)=16 nr=1
//                        → 0x8010_6d01
//  MONOOS_IOC_PERM_SET   = _IOW('m', 2, struct monoos_perm_req)
//                        direction=WRITE(1) size=16 nr=2
//                        → 0x4010_6d02
//
//  Linux _IO macro encoding (x86-64 / arm64):
//    bits 31-30: direction  (00=none, 10=read, 01=write, 11=read+write)
//    bits 29-16: size in bytes
//    bits 15- 8: magic type
//    bits  7- 0: command number
//
//  sizeof(PermReq) = 4 (pid i32) + 4 (perm u32) + 4 (val i32) + 4 (result i32) = 16
// ─────────────────────────────────────────────────────────────────────────────

const MONOOS_IOC_PERM_CHECK: u64 = 0x8010_6d01;
const MONOOS_IOC_PERM_SET:   u64 = 0x4010_6d02;

const DEV_MONOOS: &str = "/dev/monoos";

// ─────────────────────────────────────────────────────────────────────────────
//  Wire-format struct — layout must be identical to `struct monoos_perm_req`
//  in syscalls.c.  `#[repr(C)]` guarantees no Rust-side reordering or padding.
// ─────────────────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Default)]
struct PermReq {
    pid:    i32,  // target process id
    perm:   u32,  // permission bit (MONOOS_PERM_* constant)
    val:    i32,  // SET: 1 = grant, 0 = revoke
    result: i32,  // CHECK: filled by kernel (1=granted, 0=denied)
}

// ─────────────────────────────────────────────────────────────────────────────
//  Raw ioctl wrappers
// ─────────────────────────────────────────────────────────────────────────────

/// `ioctl(fd, MONOOS_IOC_PERM_SET, req)` — grant or revoke a permission.
///
/// # Safety
/// `req` must be valid for the duration of the call; `fd` must be an open
/// `/dev/monoos` file descriptor.
unsafe fn raw_perm_set(fd: std::os::unix::io::RawFd, req: *mut PermReq) -> std::io::Result<()> {
    let ret = libc::ioctl(fd, MONOOS_IOC_PERM_SET as libc::c_ulong, req);
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// `ioctl(fd, MONOOS_IOC_PERM_CHECK, req)` — query a permission.
///
/// # Safety
/// Same as `raw_perm_set`.
unsafe fn raw_perm_check(fd: std::os::unix::io::RawFd, req: *mut PermReq) -> std::io::Result<()> {
    let ret = libc::ioctl(fd, MONOOS_IOC_PERM_CHECK as libc::c_ulong, req);
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Public API
// ─────────────────────────────────────────────────────────────────────────────

/// A live connection to `/dev/monoos`.
///
/// Opened once by the permission service at startup and held for the process
/// lifetime.  All methods are `&self` so the bridge can be shared across
/// threads via `Arc<KernelBridge>`.
pub struct KernelBridge {
    fd: std::fs::File,
}

impl KernelBridge {
    /// Open `/dev/monoos` for read+write.  Fails with `ENOENT` if the
    /// kernel module is not loaded, `EACCES` if the caller lacks privilege.
    pub fn open() -> std::io::Result<Self> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DEV_MONOOS)?;
        Ok(KernelBridge { fd })
    }

    /// Grant `perm_bit` to process `pid` in the kernel permission table.
    ///
    /// On success the LSM kretprobes that call `monoos_proc_has_perm(pid, bit)`
    /// will see this grant immediately — no restart needed.
    pub fn grant(&self, pid: i32, perm_bit: u32) -> std::io::Result<()> {
        let mut req = PermReq { pid, perm: perm_bit, val: 1, result: 0 };
        unsafe { raw_perm_set(self.fd.as_raw_fd(), &mut req) }
    }

    /// Revoke `perm_bit` from process `pid`.
    pub fn revoke(&self, pid: i32, perm_bit: u32) -> std::io::Result<()> {
        let mut req = PermReq { pid, perm: perm_bit, val: 0, result: 0 };
        unsafe { raw_perm_set(self.fd.as_raw_fd(), &mut req) }
    }

    /// Query whether `pid` holds `perm_bit` according to the kernel table.
    ///
    /// Returns `true` if the kernel considers the permission granted, `false`
    /// if denied.  Does not consult the in-memory `PermissionManager` — this
    /// is a direct kernel read, useful for auditing or double-checking state.
    pub fn check(&self, pid: i32, perm_bit: u32) -> std::io::Result<bool> {
        let mut req = PermReq { pid, perm: perm_bit, val: 0, result: 0 };
        unsafe { raw_perm_check(self.fd.as_raw_fd(), &mut req) }?;
        Ok(req.result == 1)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
//
//  These tests are integration-level: they require /dev/monoos to exist.
//  They're gated behind the `kernel_integration` feature so the normal
//  `cargo test` run (host, no kernel) doesn't try to open a device node.
//
//  Run on QEMU after loading modules:
//    cargo test --features kernel_integration
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "kernel_integration"))]
mod tests {
    use super::*;

    fn bridge() -> KernelBridge {
        KernelBridge::open().expect("/dev/monoos not available — load kernel modules first")
    }

    #[test]
    fn open_device() {
        let _b = bridge();
    }

    #[test]
    fn grant_and_check() {
        let b = bridge();
        let test_pid = std::process::id() as i32;
        let camera_bit = 0x0001u32;

        b.grant(test_pid, camera_bit).expect("grant failed");
        let granted = b.check(test_pid, camera_bit).expect("check failed");
        assert!(granted, "kernel should report camera granted");

        // Clean up: revoke so we don't pollute the kernel table.
        b.revoke(test_pid, camera_bit).expect("revoke failed");
        let still_granted = b.check(test_pid, camera_bit).expect("check after revoke failed");
        assert!(!still_granted, "kernel should report camera revoked");
    }

    #[test]
    fn revoke_clears_permission() {
        let b = bridge();
        let test_pid = std::process::id() as i32;
        let mic_bit = 0x0002u32;

        b.grant(test_pid, mic_bit).unwrap();
        b.revoke(test_pid, mic_bit).unwrap();
        assert!(!b.check(test_pid, mic_bit).unwrap());
    }

    #[test]
    fn check_unset_permission_returns_false() {
        let b = bridge();
        // Use a PID that almost certainly doesn't exist and was never granted.
        let fake_pid = 99999i32;
        let result = b.check(fake_pid, 0x0001);
        // Either false (not granted) or an error (no such process) is fine.
        match result {
            Ok(false) | Err(_) => {}
            Ok(true) => panic!("unexpected: PID 99999 should not have camera"),
        }
    }
}
