<div align="center">

# SparkDns

Cross-platform DNS management and optimization desktop app for Windows, Linux, and macOS.

[![Version](https://img.shields.io/badge/version-0.1.0-ff6b35?style=flat-square)](https://github.com/Code-Leafy/SparkDns)
[![Rust](https://img.shields.io/badge/rust-1.70+-ff6b35?style=flat-square)](https://rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-2.x-ff6b35?style=flat-square)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-ff6b35?style=flat-square)](https://github.com/Code-Leafy/SparkDns/blob/main/LICENSE)
[![Status](https://img.shields.io/badge/status-active-4ec9b0?style=flat-square)]()

</div>

---

<div align="center">

<img src="icon.png" alt="SparkDns Logo" width="120" style="border-radius: 20px;">

</div>

<br>

## Overview

SparkDns is a lightweight, native desktop application for managing and optimizing DNS settings across Windows, Linux, and macOS. Built with Tauri 2 (Rust backend + TypeScript frontend), it provides real-time DNS switching, diagnostics, and system tools without the memory overhead of Electron.

> **Privacy First:** All DNS operations run locally on your machine. No data is sent to external servers.

---

### Core Features

#### ⚡ One-Click DNS Switching
Switch between pre-configured DNS providers (Cloudflare, Google, Quad9, AdGuard, Mullvad, OpenDNS) or create custom profiles with IPv4/IPv6 support.

#### 🔍 Real-Time Diagnostics
Run comprehensive DNS diagnostics with latency testing, DNSSEC validation, leak detection, and reachability probes against configurable targets.

#### 🛠️ System Tools
Flush DNS cache, renew DHCP, reset network adapters, and run traceroute — all from one panel.

#### 🔄 Auto-Switch
Automatically switch DNS profiles based on network conditions or triggers.

#### 📦 Import/Export
Backup and restore your DNS configuration with JSON export/import.

#### 🎨 Dark & Light Theme
System-aware theme with manual toggle support.

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org) (v18+)
- [Rust](https://rustup.rs) (1.70+)
- [Tauri CLI](https://tauri.app) (`cargo install tauri-cli`)

### Development

```bash
# Clone the repository
git clone https://github.com/Code-Leafy/SparkDns.git
cd SparkDns

# Install dependencies
npm install

# Start development server
npm run tauri:dev
```

### Build

```bash
# Build for production
npm run tauri:build
```

The installer will be located in `src-tauri/target/release/bundle/`.

---

## Project Structure

```text
SparkDns/
├── src/                          # TypeScript frontend
│   ├── main.ts                   # App bootstrap and view routing
│   ├── api.ts                    # Tauri command wrappers
│   ├── state.ts                  # Config state management
│   ├── types.ts                  # TypeScript interfaces
│   ├── defaults.ts               # DNS presets and default config
│   ├── styles.css                # App styles
│   ├── update.ts                 # Auto-update logic
│   ├── ui/                       # UI components (toast, dialog, etc.)
│   ├── views/                    # View modules (home, profiles, etc.)
│   └── utils/                    # Validation and helpers
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # Tauri configuration
│   ├── src/
│   │   ├── main.rs               # Tauri command registration
│   │   ├── lib.rs                # Library exports
│   │   ├── models.rs             # Shared data models
│   │   ├── config.rs             # Config persistence
│   │   ├── platform.rs           # OS capability detection
│   │   ├── diagnostics.rs        # DNS diagnostics
│   │   ├── validation.rs         # Input validation
│   │   ├── errors.rs             # Error types
│   │   ├── elevation.rs          # Privilege elevation
│   │   ├── command_runner.rs      # Safe command execution
│   │   ├── process_watcher.rs    # Process monitoring
│   │   └── dns/                  # OS-specific DNS backends
│   │       ├── mod.rs
│   │       ├── windows.rs
│   │       ├── macos.rs
│   │       └── linux.rs
│   └── icons/                    # App icons
├── package.json
├── tsconfig.json
├── vite.config.ts
└── index.html                    # Vite entry point
```

---

## Supported Platforms

| Feature | Windows | Linux | macOS |
|---------|---------|-------|-------|
| DNS Switching | ✅ | ✅ | ✅ |
| DNS Cache Flush | ✅ | ✅ | ✅ |
| DHCP Renew | ✅ | ✅ (NetworkManager) | ❌ |
| Adapter Reset | ✅ | ❌ | ❌ |
| Traceroute | ✅ | ✅ | ✅ |
| DNS-over-HTTPS | ✅ | ❌ | ✅ |
| System Tray | ✅ | ✅ | ✅ |
| Auto-Start | ✅ | ✅ | ✅ |

---

## Built-in DNS Providers

| Provider | IPv4 | IPv6 | DoH |
|----------|------|------|-----|
| Cloudflare | 1.1.1.1 | 2606:4700:4700::1111 | ✅ |
| Google | 8.8.8.8 | 2001:4860:4860::8888 | ✅ |
| Quad9 | 9.9.9.9 | 2620:fe::fe | ✅ |
| AdGuard | 94.140.14.14 | 2a10:50c0::ad1:ff | ✅ |
| Mullvad | 194.242.2.2 | 2a07:a4c0::2 | ✅ |
| OpenDNS | 208.67.222.222 | 2620:119:35::35 | ❌ |

---

<details>
<summary><kbd>❓</kbd> FAQ & Troubleshooting</summary>

**Why does DNS switching require elevation?**
Modifying system DNS settings is a privileged operation on all operating systems. SparkDns will prompt for elevation when needed.

**Does SparkDns work on ARM devices?**
SparkDns supports x86_64 on all platforms. ARM support depends on Tauri and Rust target availability.

**Is my DNS traffic encrypted?**
SparkDns configures your system DNS servers. For encrypted DNS, enable DNS-over-HTTPS in settings (Windows and macOS only).

</details>

<br>

<div align="center">

> **⚠️ Educational Purpose Only:** This project is provided for educational and research purposes. Users are solely responsible for compliance with all local laws.

[MIT License](https://github.com/Code-Leafy/SparkDns/blob/main/LICENSE) · Crafted by [Code-Leafy](https://github.com/Code-Leafy)

</div>
