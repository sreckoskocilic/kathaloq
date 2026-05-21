# Kathaloq

[![CI](https://github.com/sreckoskocilic/kathaloq/actions/workflows/ci.yml/badge.svg)](https://github.com/sreckoskocilic/kathaloq/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)

Indexes folders (external drives, NAS mounts, whatever) into a local SQLite database. Once indexed, you can search and browse the file tree even after unplugging the drive.

Built with Tauri 2, Svelte 5, and Rust.

## Prerequisites

- [Node.js](https://nodejs.org/) v20+
- [Rust](https://www.rust-lang.org/tools/install) stable
- Tauri system deps for your OS: [tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/)

On macOS you also need Xcode CLI tools:

```bash
xcode-select --install
```

On Debian/Ubuntu:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

On Windows, install [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/). WebView2 ships with Windows 11; on 10 you may need to install it separately.

## Build from source

```bash
git clone https://github.com/sreckoskocilic/kathaloq.git
cd kathaloq
npm install
npm run tauri build
```

The built app ends up in `src-tauri/target/release/bundle/` — `.app`/`.dmg` on macOS, `.deb`/`.appimage` on Linux, `.msi`/`.nsis` on Windows.

## Development

```bash
npm run serve
```

Runs Vite + the Rust backend together with hot reload.

## Test and lint

```bash
npm test                              # vitest
cd src-tauri && cargo test            # rust

npm run lint                          # eslint + clippy
npm run format                        # prettier
```
