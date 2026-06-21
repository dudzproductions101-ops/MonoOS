# MonoOS SDK – Getting Started

**Version:** 1.0.0  
**Author:** DudasCorp  
**Target platform:** ARM64 Android-compatible devices running MonoOS 1.x

---

## Prerequisites

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust | 1.78 | `rustup install stable` |
| Clang/LLVM | 17 | `apt install clang-17` |
| CMake | 3.28 | `apt install cmake` |
| Python | 3.11 | `apt install python3` |
| MonoOS SDK | 1.0.0 | See §2 |

---

## 1. Installing the SDK

```bash
# Clone the SDK tools
git clone https://github.com/DudasCorp/monoos-sdk.git ~/.monoos-sdk
echo 'export MONOOS_SDK_HOME="$HOME/.monoos-sdk"' >> ~/.bashrc
echo 'export PATH="$MONOOS_SDK_HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify installation
monoos-sdk --version   # should print 1.0.0
```

---

## 2. Creating Your First App

```bash
monoos-sdk new com.example.hello --lang rust
cd hello
monoos-sdk build --target arm64-v8a
monoos-sdk install --device auto
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
name         = "Hello MonoOS"
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
use monoos_sdk::permissions::{Permission, request_permission};

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
monoos-sdk build --release --sign ~/.keys/mykey.p12
```

The output is an `.opk` file in `build/release/`.

---

## 6. Publishing

Submit your `.opk` to the MonoOS Package Repository:

```bash
monoos-sdk publish build/release/com.example.hello-1.0.0.opk \
    --api-key $MONOOS_DEV_KEY
```

---

## Further Reading

- `sdk/documentation/api_reference.md` – full API reference
- `sdk/documentation/security_guide.md` – security best practices
- `sdk/templates/` – project templates for common app types
