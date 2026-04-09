# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

civlink is a WebRTC-based web application for remote control of Icom amateur radios (IC-705, IC-7300, etc.). It provides a browser frontend for radio operation including audio, spectrum display, and rig control over the network.

## Architecture

- **Backend** — Rust, using `axum` for HTTP/WebSocket serving and `webrtc-rs` for WebRTC media transport
- **Frontend** — TypeScript (not JavaScript), served as static assets by axum
- **Radio control** — Delegated to the `sidebridge` crate in the parent workspace, which handles Icom CI-V protocol and audio I/O
- **Error handling** — `thiserror` for typed backend errors
- **CLI** — `clap` for command-line argument parsing

### Data flow

```
Browser (TypeScript) <--WebRTC--> axum backend <--sidebridge--> Icom radio (CI-V over serial/TCP)
```

- Audio: bidirectional via WebRTC media tracks
- Spectrum: backend computes and pushes to frontend
- Rig control (frequency, mode, PTT): frontend sends commands via WebRTC data channel or WebSocket, backend translates to CI-V via sidebridge
- Multiple listeners can connect simultaneously (receive audio and spectrum), but only one client may have control (rig commands, TX) at a time

## System Dependencies

Backend requires ALSA headers and cmake (for opus):

```bash
# Debian/Ubuntu
sudo apt install libasound2-dev cmake

# Guix
guix install alsa-lib cmake
```

## Build Commands

```bash
cargo build -p civlink               # build backend
cargo test -p civlink                 # run tests
```

### Frontend

TypeScript + SolidJS frontend built with Vite. Sources live in `frontend/`.

```bash
cd frontend && npm install            # install dependencies
npm run dev                           # vite dev server
npm run build                         # production build to dist/
```

## Safety

- **Never put the radio in TX (transmit) during development.** Always ask before sending any PTT or TX command.

## Deployment

Target is a Raspberry Pi (hostname `shack`) with an Icom IC-705 connected. USB audio is card 3 (`USB Audio CODEC`). Use `aplay`/`arecord` on shack to verify audio. See the workspace root `Justfile` for cross-compile and deploy patterns:

```bash
cross build -p civlink --target aarch64-unknown-linux-gnu --release
scp target/aarch64-unknown-linux-gnu/release/civlink shack:~/
```

## Key Dependencies

| Crate       | Purpose                          |
|-------------|----------------------------------|
| axum        | HTTP server, WebSocket, static files |
| webrtc-rs   | WebRTC peer connections and media |
| cpal        | Audio device capture and playback |
| opus        | Opus audio codec encoding/decoding |
| sidebridge  | Radio control (CI-V protocol)    |
| thiserror   | Error type definitions           |
| clap        | CLI argument parsing             |
| tokio       | Async runtime                    |
| pool        | Buffer pool from workspace — use for allocation-free audio buffer reuse |

## Style

- One struct per file, named in snake_case after the struct (e.g. `AudioStream` lives in `audio_stream.rs`)
- Use `foo.rs` + `foo/` for submodules, not `mod.rs`
- Frontend code must be TypeScript with strict mode enabled
- UI design inspired by Icom RS-BA1 (radio control panel layout, spectrum/waterfall, VFO display) but with a modern, clean, functional aesthetic
- Backend errors use `thiserror` derive macros, not ad-hoc strings
- Configuration files use TOML format
- General functionality goes in `lib.rs` so it can be exercised by integration tests; `main.rs` should be a thin entry point
- Prefer functional style: iterators, combinators, and transformations over imperative loops
- Use message passing (channels) for communication between components rather than shared mutable state
- Code must remain idiomatic Rust — don't fight the type system or borrow checker
- Reuse workspace crates (`pool`, `spectrum`, `rtp`, `doublemap`, etc.) rather than reimplementing functionality
