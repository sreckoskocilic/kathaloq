# Kathaloq

[![CI](https://github.com/sreckoskocilic/kathaloq/actions/workflows/ci.yml/badge.svg)](https://github.com/sreckoskocilic/kathaloq/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Local catalog browser for attached directories (removable drives, NAS mounts, etc). Indexes file trees into SQLite so you can search and browse them after the disk is disconnected.

Tauri 2 + Svelte 5 + Rust.

## Prerequisites

- [Node.js](https://nodejs.org/) (v20+)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Tauri system dependencies for your OS — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### macOS

Xcode Command Line Tools:

```bash
xcode-select --install
```

### Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### Windows

- [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 11)

## Build from source

```bash
git clone https://github.com/sreckoskocilic/kathaloq.git
cd kathaloq
npm install
npm run tauri build
```

Output:
- macOS: `src-tauri/target/release/bundle/macos/Kathaloq.app` and `.dmg`
- Linux: `src-tauri/target/release/bundle/deb/` and `appimage/`
- Windows: `src-tauri/target/release/bundle/msi/` and `nsis/`

## Development

```bash
npm run serve
```

Starts frontend dev server + Rust backend with hot reload.

## Test

```bash
npm test                              # Frontend (vitest)
cd src-tauri && cargo test            # Backend (Rust)
```

## Lint

```bash
npm run lint                          # ESLint + Clippy
npm run format                        # Prettier
```
