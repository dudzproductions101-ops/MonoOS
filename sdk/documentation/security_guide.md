# OneOS SDK – Security Guide

**SDK Version:** 1.0.0

---

## 1. Principle of Least Privilege

Declare only the permissions your app genuinely needs.  The runtime  
permission system shows the user exactly what each permission grants.  
Unnecessary permissions reduce user trust and increase review friction  
on the package repository.

**Bad:**
```toml
[[permissions]]
name = "STORAGE"   # declared "just in case"
```

**Good:**
```toml
[[permissions]]
name = "STORAGE"
reason = "Required to save exported PDF files."
```

---

## 2. Secure Data Storage

Never store sensitive data (keys, tokens, passwords) in:
- Shared preferences (world-readable on unencrypted devices)
- External storage (`/sdcard/`)
- Log files

Use the **OneOS KeyStore** instead:

```rust
use oneos_sdk::keystore::{KeyStore, KeySpec};

let ks = KeyStore::open("com.example.app").unwrap();
ks.store_secret("api_token", token.as_bytes()).unwrap();
```

Keys stored via KeyStore are encrypted with a device-specific key backed  
by the Trusted Execution Environment (TEE).

---

## 3. Network Security

### Enforce TLS

All outbound connections must use TLS 1.2 or higher.  The OneOS network  
stack rejects plain-text connections by default unless the app explicitly  
declares a cleartext exception in its manifest:

```toml
[network]
cleartext_allowed = false   # default; do not change unless absolutely necessary
```

### Certificate Pinning

```rust
use oneos_sdk::network::HttpClient;

let client = HttpClient::builder()
    .pin_certificate("sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
    .build();
```

### DNS-over-HTTPS

All DNS queries issued via `oneos_net_resolve()` automatically go through  
the system DoH resolver.  Bypass is not permitted for third-party apps.

---

## 4. IPC Security

Binder services exposed by your app should validate the caller's  
permission on every call:

```rust
fn handle_request(&self, caller_uid: u32, request: &Request) -> Result<Response> {
    PermissionManager::check(caller_uid, Permission::Camera)?;
    // proceed only if check() returns Ok
}
```

---

## 5. Content Providers

Set `exported = false` on any content provider that should only be  
accessible by your own app:

```toml
[[providers]]
name     = "com.example.app.LocalDataProvider"
exported = false
```

---

## 6. Dependency Auditing

Run `oneos-sdk audit` before each release.  It checks your Cargo.lock  
for known CVEs via the RustSec Advisory Database and flags transitive  
dependencies with known vulnerabilities.

```bash
oneos-sdk audit
# WARN cve-2024-XXXXX in base64 0.13.0 – update to 0.21+
```

---

## 7. Release Signing

Sign your release build with a 4096-bit RSA or P-256 ECDSA key.  Never  
commit your signing key to version control.

```bash
oneos-sdk build --release --sign ~/.keys/release.p12
```

Store your key in a hardware security key (YubiKey) or a secrets manager  
(Vault, AWS Secrets Manager) for production releases.
