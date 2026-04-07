# CLAUDE.md — sidebridge

Async radio control abstraction layer for amateur radio transceivers, authored by SM6WJM.

## Build

```bash
cargo build -p sidebridge
cargo test -p sidebridge
```

## Architecture

- `traits.rs` — Modular trait hierarchy and types:
  - `Radio` — core async trait (frequency, mode, PTT)
  - `RadioInfo` — model name and capability discovery
  - `RadioMeter` — S-meter, SWR, ALC, RF power
  - `RadioScope` — spectrum/waterfall streaming
  - Types: `Mode`, `Capabilities`, `ScopeFrame`, `RadioError`
- `drivers/` — Radio driver implementations (future home of per-radio drivers)
- `lib.rs` — Re-exports all public types

## Key patterns

- Drivers implement only the traits their hardware supports (e.g. IC-781 skips `RadioScope`)
- Protocol-specific code (CI-V parsing, CAT commands) belongs in driver crates, not here
- Symmetric get/set pairs: `frequency()` / `set_frequency()`, not monolithic state snapshots
- All traits require `Send + Sync` for use across async tasks

## Reference docs

The `docs/` directory contains manufacturer protocol references:

- `IC-705_ENG_CI-V_1_20200721.pdf` — Icom IC-705 CI-V Reference Guide. Full command table, CI-V frame format, BCD frequency encoding, scope data commands.
- `FT-891_CAT_OM_ENG_1909-C.pdf` — Yaesu FT-891 CAT Operation Reference. ASCII-based protocol with two-letter commands and semicolon terminator.

Use `rga` (ripgrep-all) for searching inside the PDF docs:

```bash
rga "frequency" sidebridge/docs/
rga "scope" sidebridge/docs/IC-705_ENG_CI-V_1_20200721.pdf
```

## Dependencies

- `async-trait` — async fn in traits
- `serde` — serialization for `Mode`, `ScopeFrame`
- `thiserror` — error derive for `RadioError`
- `tokio-stream` — `Stream` trait for `RadioScope`
