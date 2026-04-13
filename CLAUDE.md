# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build                          # build entire workspace
cargo build -p filters               # build a single crate
cargo test                           # run all tests (some airspyhf tests need hardware, marked #[ignore])
cargo test -p signals                # test a single crate
cargo test test_oscillator           # run a single test by name
cargo bench -p filters               # run benchmarks for a crate
cross build --target aarch64-unknown-linux-gnu  # cross-compile for Raspberry Pi 4/5
```

## Architecture

This is a Rust workspace for software-defined radio (SDR) and amateur radio control, authored by SM6WJM.

### Signal processing pipeline (SDR receiver path)

- **airspyhf-sys** — Raw FFI bindings to `libairspyhf` (generated via `bindgen`, requires the C library installed)
- **airspyhf** — Safe Rust wrapper around `airspyhf-sys`. Provides `Device` with callback-based IQ sample streaming (`Complex32`)
- **signals** — Signal sources: complex oscillators (standard, fast/SIMD-friendly, Fs/4-optimized), file source, noise source. All implement `Iterator<Item = Complex32>`
- **filters** — FIR filters, decimation chains, and `ComplexRotator` frequency shifting for `Complex32`. Three FIR tiers: `NaiveFirFilter` (VecDeque) → `DynFirFilter` (RingBuffer) → `FirFilter` (const generic), `ChainableDecimator` for pipeline composition, canonical `dot_product`, and pre-computed kernels
- **doublemap** — Lock-free ring buffer using Linux `memfd_create` + double memory mapping (via `nix` crate) for zero-copy `Producer`/`Consumer` IPC with `Disconnected` error handling
- **pool** — Buffer pool and channel-based buffer pool for allocation-free real-time paths. RAII `BufferGuard` auto-returns buffers on drop
- **spectrum** — FFT-based spectrum analyzer (windowing + power density), used by the receiver's spectrum server
- **receiver** — Main SDR receiver binary. Streams IQ from AirspyHF, computes spectrum via `doublemap` ring buffer, serves spectrum data over a Unix domain socket (`/tmp/echo.sock`)

### Audio streaming

- **rtp** — RTP header construction for network audio transport
- **sideband** — Captures audio via `cpal`, encodes with Opus, streams as RTP/UDP packets to a remote address

### Radio control (Icom CI-V)

- **sidebridge** — Async radio control abstraction. Modular trait hierarchy: `Radio` (frequency/mode/PTT), `RadioInfo` (capabilities), `RadioMeter` (S-meter/SWR/ALC), `RadioScope` (spectrum). Includes Icom CI-V parser/codec (`nom`-based), `Transport` (URL-based serial/TCP connectivity), and `serial-bridge`/`civ-client`/`civ-poll` binaries

## Style

- Keep types, enums, and impls in their own files — one concept per file, named after the type (e.g. `transport.rs` for `Transport`)
- Use `foo.rs` + `foo/` for submodules, not `mod.rs`

## Key Patterns

- Complex IQ samples use `num_complex::Complex32` throughout
- Real-time audio/sample paths avoid allocation using `pool::BufferPool` and `doublemap` ring buffers
- Async crates (`sidebridge`) use `tokio`; signal processing crates are synchronous
- `airspyhf-sys` requires `libairspyhf` headers and library at build time; hardware-dependent tests are `#[ignore]`
- POSIX syscalls (mmap, memfd_create, ftruncate) use the `nix` crate for safe wrappers, not raw `libc`
- The CI-V parser in `sidebridge` (under `drivers/icom/civ`) uses `nom` for frame parsing
- Radio protocol docs (CI-V, CAT PDFs) live in `sidebridge/docs/` — use `rga` (ripgrep-all) to search them: `rga "frequency" sidebridge/docs/`
