# Kathaloq

[![CI](https://github.com/sreckoskocilic/kathaloq/actions/workflows/ci.yml/badge.svg)](https://github.com/sreckoskocilic/kathaloq/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Indexes folders — external drives, NAS mounts, whatever — into a local SQLite database, so you can search and browse the file tree after unplugging the drive.

Tauri 2 + Svelte 5 + Rust. Developed on macOS; the code is platform-agnostic and CI builds and tests it on Linux, so it should build anywhere Tauri does. Only the macOS build is actually used.

## Prerequisites

- [Node.js](https://nodejs.org/) 24 or 26
- [Rust](https://www.rust-lang.org/tools/install) 1.85+ (edition 2024)
- Tauri system dependencies: [v2.tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/)

macOS also needs `xcode-select --install`. Debian/Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

## Run

```bash
npm install
npm run serve
```

## Build

```bash
npm run tauri build
```

Output in `src-tauri/target/release/bundle/`.

## Checks

```bash
npm run check         # svelte-check + tsc
npm run lint          # eslint + clippy
npm run format:check  # prettier
npm test              # vitest
cargo test --manifest-path src-tauri/Cargo.toml
```

## Data

Everything lives in one SQLite file. On macOS:

```
~/Library/Application Support/com.kathaloq.desktop/kathaloq.db
```

## License

MIT
