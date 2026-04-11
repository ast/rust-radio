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

## Multi-radio support

The Icom CI-V driver should work with both IC-705 and IC-7300 (including MK2) without code changes. The CI-V protocol is nearly identical between models — same command numbers, same BCD encoding, same scope data format. Key differences:

- **CI-V address**: IC-705 = `0xA4`, IC-7300/MK2 = `0xB6` (currently hardcoded, needs to become configurable)
- **Scope modes**: IC-7300 adds SCROLL-C (0x02) and SCROLL-F (0x03) beyond Center/Fixed
- **Scope bins**: Both use 475 bins, range 0–160, same division structure (LAN=1, USB=11)
- **Scope spans**: Identical values (2.5k–500k) and encoding (command 27 15)

## Reference docs

The `docs/` directory contains manufacturer protocol references:

- `IC-705_ENG_CI-V_1_20200721.pdf` — Icom IC-705 CI-V Reference Guide. Full command table, CI-V frame format, BCD frequency encoding, scope data commands.
- `IC-7300MK2_ENG_CI-V_0.pdf` — Icom IC-7300MK2 CI-V Reference Guide. Very similar to IC-705; has additional scope modes (SCROLL-C/F) and some extra commands (VBW, data filter width).
- `FT-891_CAT_OM_ENG_1909-C.pdf` — Yaesu FT-891 CAT Operation Reference. ASCII-based protocol with two-letter commands and semicolon terminator.

Use `rga` (ripgrep-all) for searching inside the PDF docs:

```bash
rga "frequency" sidebridge/docs/
rga "scope" sidebridge/docs/IC-705_ENG_CI-V_1_20200721.pdf
rga "scope" sidebridge/docs/IC-7300MK2_ENG_CI-V_0.pdf
```

## Dependencies

- `async-trait` — async fn in traits
- `serde` — serialization for `Mode`, `ScopeFrame`
- `thiserror` — error derive for `RadioError`
- `tokio-stream` — `Stream` trait for `RadioScope`
