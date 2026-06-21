# OneOS SDK – Getting Started

**Version:** 1.0.0  
**Author:** DudasCorp  
**Target platform:** ARM64 Android-compatible devices running OneOS 1.x

---

## Prerequisites

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust | 1.78 | `rustup install stable` |
| Clang/LLVM | 17 | `apt install clang-17` |
| CMake | 3.28 | `apt install cmake` |
| Python | 3.11 | `apt install python3` |
| OneOS SDK | 1.0.0 | See §2 |

---

## 1. Installing the SDK

```bash
# Clone the SDK tools
git clone https://github.com/DudasCorp/oneos-sdk.git ~/.oneos-sdk
echo 'export ONEOS_SDK_HOME="$HOME/.oneos-sdk"' >> ~/.bashrc
echo 'export PATH="$ONEOS_SDK_HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify installation
oneos-sdk --version   # should print 1.0.0
```

---

## 2. Creating Your First App

```bash
oneos-sdk new com.example.hello --lang rust
cd hello
oneos-sdk build --target arm64-v8a
oneos-sdk install --device auto
```

### Project layout

```
hello/
├── META-INF/
│   └── manifest.toml      # package metadata
├── src/
│   └── main.rs            # application entry point
├── res/
│   ├── icons/             # app icons (svg recommended)
│   └── qml/               # UI files
└── Cargo.toml
```

---

## 3. Manifest Format

`META-INF/manifest.toml` example:

```toml
[package]
name         = "Hello OneOS"
package_name = "com.example.hello"
version_name = "1.0.0"
version_code = 1
min_sdk      = 1
target_sdk   = 1
label        = "Hello"
icon         = "res/icons/app_icon.svg"

[[permissions]]
name = "CAMERA"

[[permissions]]
name = "MICROPHONE"

[[activities]]
name    = "com.example.hello.MainActivity"
label   = "Hello"
main    = true
exported = true
```

---

## 4. Requesting Permissions

```rust
use oneos_sdk::permissions::{Permission, request_permission};

fn on_launch() {
    request_permission(Permission::Camera, |granted| {
        if granted {
            println!("Camera access granted!");
        }
    });
}
```

---

## 5. Building for Release

```bash
oneos-sdk build --release --sign ~/.keys/mykey.p12
```

The output is an `.opk` file in `build/release/`.

---

## 6. Publishing

Submit your `.opk` to the OneOS Package Repository:

```bash
oneos-sdk publish build/release/com.example.hello-1.0.0.opk \
    --api-key $ONEOS_DEV_KEY
```

---

## Further Reading

- `sdk/documentation/api_reference.md` – full API reference
- `sdk/documentation/security_guide.md` – security best practices
- `sdk/templates/` – project templates for common app types
