# OneOS

**by DudasCorp**

A privacy-focused, security-first mobile operating system built on the Linux kernel.

> **Early Development:** OneOS is currently an experimental project focused on learning operating system development, mobile architecture, low-level programming, and system design.

---

## About

I've always wanted to learn how to build an operating system from scratch, and I decided this summer was the perfect time to finally start. With plenty of free time, curiosity, and a questionable amount of sanity i started making OneOS with my four friends.

OneOS is an open-source mobile operating system designed around privacy, security, transparency, and user control. The goal is to create a modern smartphone OS that minimizes telemetry, maximizes security, and gives users full visibility into how their device works.

Built on the Linux kernel, OneOS combines modern mobile technologies with a security-first architecture, providing a platform that is modular, customizable, and developer-friendly.

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

```text
Apps
 ↓
OneUI
 ↓
OneFramework
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

```text
OneOS/
├── boot/
├── kernel/
├── drivers/
├── hal/
├── firmware/
├── init/
├── system/
├── services/
├── framework/
├── security/
├── privacy/
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

* Sell OneOS
* Commercialize OneOS
* Sell modified versions of OneOS
* Claim ownership of OneOS
* Remove attribution to the original authors
* Use OneOS or its source code for AI training, dataset creation, or machine learning purposes

---

## Current Status

OneOS is not yet a functional operating system. The project is currently focused on architecture design, research, tooling, and infrastructure before implementation begins.

---

## License

OneOS is distributed under the OneOS Non-Commercial License.

See the LICENSE file for complete terms.

---

*Building a phone OS, one mistake at a time.*
