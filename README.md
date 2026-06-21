# MonoOS

**by DudasCorp**

A privacy-focused, security-first mobile operating system built on the Linux kernel.

> **Early Development:** MonoOS is currently an experimental project focused on learning operating system development, mobile architecture, low-level programming, and system design.

---

## About

I've always wanted to learn how to build an operating system from scratch, and I decided this summer was the perfect time to finally start. With plenty of free time, curiosity, and a questionable amount of sanity i started making MonoOS with my four friends.

MonoOS is an open-source mobile operating system designed around privacy, security, transparency, and user control. The goal is to create a modern smartphone OS that minimizes telemetry, maximizes security, and gives users full visibility into how their device works.

Built on the Linux kernel, MonoOS combines modern mobile technologies with a security-first architecture, providing a platform that is modular, customizable, and developer-friendly.

This project is also a personal learning journey into:

* Operating system development
* Linux internals
* Mobile architectures
* Security engineering
* Systems programming
* UI/UX design
* Software architecture

---

## Goals

* Privacy by default
* No mandatory telemetry
* Open development
* Modern mobile experience
* Strong security model
* Developer-friendly ecosystem
* Modular architecture
* Long-term maintainability
* User control and customization

---

## Tech Stack

### Core Languages

* Rust
* C++
* C
* QML

### Tooling

* Python
* Bash
* Cargo
* CMake
* Ninja
* Git

### Core Technologies

* Linux Kernel (LTS)
* Wayland
* ARM64
* SQLite
* Vulkan
* OpenGL ES

---

## Planned Architecture

```
Apps
 ↓
MonoUI
 ↓
MonoFramework
 ↓
System Services
 ↓
Hardware Abstraction Layer
 ↓
Linux Kernel
 ↓
Hardware
```

---

## Project Structure

```
MonoOS/
├── boot/
├── kernel/
├── drivers/
├── hal/
├── init/
├── services/
├── framework/
├── security/        (includes security/crypto, the encryption layer)
├── packages/         (OPK package manager + installer)
├── networking/
├── telephony/
├── multimedia/
├── ui/
├── apps/
├── sdk/
├── build/
├── testing/
├── tools/
└── docs/
```

See `docs/architecture.md` for what each directory actually contains and
which parts build/test today, and `docs/roadmap.md` for what's planned
next.

---

## Permissions

You are free to:

* View the source code
* Study the source code
* Modify the source code
* Create forks
* Contribute improvements
* Use the code for educational purposes
* Use the code for personal projects

You may not:

* Sell MonoOS
* Commercialize MonoOS
* Sell modified versions of MonoOS
* Claim ownership of MonoOS
* Remove attribution to the original authors
* Use MonoOS or its source code for AI training, dataset creation, or machine learning purposes

---

## Current Status

MonoOS is not yet a bootable, functional operating system, but it has moved
past pure architecture sketching: several components now build and pass
automated tests, including the boot manager and verified-boot orchestration
logic, the full app SDK with safe Rust bindings, a real AES-256-GCM /
Ed25519 encryption layer, the OPK package manager, and the privacy engine
(tracker blocking, telemetry guarding, camera/mic/network monitors).

See `docs/architecture.md` for a directory-by-directory breakdown of what's
implemented and tested vs. still a stub, and `docs/roadmap.md` for what's
being worked on next (app store UI, real bootloader-stage crypto, Android-
parity features like intents and accessibility services, and more).

---

## License

MonoOS is distributed under the MonoOS Non-Commercial License.

See the LICENSE file for complete terms.

---

*Building a phone OS, one mistake at a time.*
