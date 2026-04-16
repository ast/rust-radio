# Deep review: filters / doublemap / pool / sdrlink

## filters

Strong crate overall — clear trait hierarchy (`Filter`/`Decimator`/`DelayLine`), good doc comments tying choices to textbooks (Fred Harris, RBJ cookbook), long-signal tests that compare optimized impls against the naive reference.

**⚠️ Convention mismatch in convolution direction.** `DynFirComplex::new` reverses `h` explicitly (dynamic_cplx.rs:15-17, with a doc comment justifying it); `DynFirFilter`, `FirFilter`, and `FirDecimator` do *not*. They compute `Σ h[i] · z[i]` where `z[0]` is the oldest sample — i.e. a correlation, not a convolution. This is invisible today because every real kernel in use (`FM_64`, `HB_35`, `windowed_sinc_lp`) is symmetric. Add an asymmetric real FIR and it'll silently produce the time-reversed response. Either make all FIRs agree on the convention, or add a loud doc/test for the "oldest-first" semantics.

**`dot_product` has pointer-arith with no debug_assert on lengths** (fir.rs:42-47). Currently safe because `len = h.len().min(z.len())` guarantees both slices are at least `len` elements. Add `debug_assert!(h.len() >= len && z.len() >= len)` as a refactor guard.

**Minor:** `RingBuffer::taps` is stored but `start()` and the trait both recompute it — dead field; `as_slice` in ringbuffer.rs:59 also recomputes `start` instead of calling `self.start()`.

## doublemap

Core trick (anonymous memfd mapped twice at adjacent virtual addresses) is implemented carefully — ZST guard, overflow checks, cleanup on second-mmap failure, `OwnedFd` keeps the memfd alive.

**⚠️ Drop may not unmap the full reserved region** (doublemap.rs:131). `aligned` is a page-size multiple, but `capacity` is `aligned / size_of_t` (integer division). If `size_of::<T>()` doesn't divide `page_size`, `capacity * 2 * size_of_t < aligned * 2` and `munmap` leaks pages. Safe for `u8/f32/Complex32` on 4K/16K pages, but latent. Either `munmap(self.ptr, aligned * 2)` using a stored `aligned`, or assert at `new()` that `page_size % size_of::<T>() == 0`.

**`as_slice()` returns a `2*capacity` slice** (doublemap.rs:109). Surprising name — a reader expects `capacity`. Consider `as_mirrored_slice` or `full_slice`. The `ringbuffer` / `thread_ring_buffer` users go through `as_ptr()` + mask anyway; this method looks like a foot-gun waiting for a new caller.

**`Producer::produce` holds the mutex across the user closure** (thread_ring_buffer.rs:106-137). For a true SPSC with split read/write indices the slice doesn't need mutex protection during the fill — releasing the lock during `f(slice)` would let producer and consumer actually run in parallel. Current design serializes them.

**No `try_produce` / `try_consume`.** Upstream (IqBroker ← airspyhf USB callback) has no way to shed load — a slow consumer blocks the callback thread, which is the wrong real-time tradeoff (see sdrlink §§). Add non-blocking variants and let the RT side drop rather than wait.

## pool

Three pool flavors, each solving a slightly different fan-out problem; `SharedBufferPool` correctly uses `Weak<Pool>` so a pool can outlive-nothing (test verifies).

**⚠️ `BufferPool` guards mutability with a `debug_assert` on Arc strong_count** (buffer_pool.rs:38-42). In release builds a buggy caller who clones the `Arc<Vec<T>>` slips it back into the pool; the next `get()` hands out an aliased buffer, and `deref_mut()` panics with `expect("buffer has outstanding clones")`. Fix: either `assert_eq!` unconditionally, or ditch `Arc<Vec<T>>` for `Vec<T>` directly (like `BufferMut`/`Shared` already does — there's no reason the "unique mutable" pool needs an Arc).

**No `try_get` / timeout.** `BufferPool::get` and `SharedBufferPool::get` both block on condvar; called from any path that's also upstream of a blocking consumer, you get the cascade described below.

**`ChannelPool::BufferGuard::drop` silently drops the buffer if the channel is closed** (channel_pool.rs:43). Not wrong given current usage (pool outlives guards), but worth a doc comment on the lifecycle contract.

## sdrlink

Clean structure: IQ broker → spectrum worker + per-client demod pipelines → WebRTC transport. Good test coverage for auth/config/session/API; fake source and integration tests make it possible to develop without hardware.

### Security (worth fixing before any public deploy)

**⚠️ Sessions never expire.** `SessionStore::remove_expired` (session.rs:49-54) is never called anywhere in the codebase; `username()` doesn't check TTL either. `SESSION_TTL = 24h` is dead code. Consequences: unbounded memory growth, tokens valid forever. Fix: spawn a periodic cleanup in `commands/serve.rs`, and/or check `created_at.elapsed() < SESSION_TTL` inline in `username()`.

**⚠️ Username enumeration via login timing** (api.rs:45-70). Unknown user returns immediately; known user runs argon2 (~100 ms). Fix: pre-hash a dummy password at startup and verify against it when the user is missing so the timing is uniform.

**WS token in query string** (signaling.rs:24-25, 73). Lands in access logs and proxy caches. Prefer `Sec-WebSocket-Protocol: bearer.TOKEN` or similar. Acceptable for LAN-only; flag for anything public.

**Password echoes at the terminal** (user_add.rs:29-35) — plain `io::stdin().read_line`. Add `rpassword` or manually toggle termios.

No login rate limiting — argon2's 100 ms per attempt is partial defense only.

### Concurrency / correctness

**⚠️ Viewport race you already know about** (signaling.rs:125-146). `current_center` is only mutated in the `center_rx.recv()` select branch; a `SetCenter` followed immediately by `SetViewport` processes the viewport against the stale center → the `invalid viewport` warning. Cheap fix: also update `current_center` inline in the `SetCenter` match arm.

**⚠️ Back-pressure cascade from the WS pump to the USB callback.** Path: WS client slow → `dc.send` slow → `spectrum_pump` stalls → `spectrum_worker` stops consuming → its `doublemap` ring fills → `IqBroker::broadcast` → `Producer::produce` condvar-waits → the AirspyHF C callback thread blocks. The USB driver will drop samples under the hood but the rest of the system stalls too. The `pool::BufferPool::get()` calls in `spectrum_worker` (lines 49, 54) create a second identical trap: one slow listener that holds on to `Shared<u8>` frames exhausts `u8_pool` → the spectrum thread blocks → same cascade. Recommend `try_produce` + frame-drop in `IqBroker::broadcast`, and `try_get` with a "skip this frame" path in `spectrum_worker`.

**Cooperative shutdown of the demod thread** (demod_pipeline.rs:105-149). `DemodHandle::drop` sets the atomic, but if the worker is parked inside `consumer.consume`, it won't notice until the next IQ block arrives. At 768k/s that's sub-5ms so it's fine in practice; if the IQ source ever stops, the thread hangs until the broker is dropped. A `Disconnected`-or-shutdown select would clean this up.

**`handle_socket` spectrum_rx guard** (signaling.rs:244-257): `Arc<tokio::Mutex<Option<Receiver>>>` just to detect "data channel opens twice". `OnceLock` or a one-shot would be less ceremony.

### Style / small stuff

- `let rx = rx;` (signaling.rs:262) is a no-op; drop it.
- `RwLock<f64>` for `center_hz` (sdr_handle.rs:27) can be an `AtomicU64` + `f64::to_bits` — lock-free and shorter.
- `tower_http::cors` in `Cargo.toml` (line 10) isn't used anywhere — either wire into `router.rs` for dev or remove.
- Hardcoded `stun:stun.l.google.com:19302` (peer.rs:29) — plumb through `SdrConfig` for deployments without Google STUN reachability.
- `static_files.rs` hand-rolls a MIME switch while `rust-embed` is already built with `mime-guess` feature (Cargo.toml:46) — `content.metadata().mimetype()` would replace lines 10-23.
- Consider generating TS types from the Rust `SignalingMessage`/`DemodState`/`Mode` (ts-rs or specta) to remove the hand-kept duplicate in `SpectrumView.tsx`.
- `Bytes::copy_from_slice` on every Opus frame / spectrum frame (demod_pipeline.rs:131, signaling.rs:304) — 50/s × N listeners, ~20/s × N listeners — probably fine but a `Bytes::from_owner` + pooled Vec path is friendlier at scale.

### Testing gaps

- `IqBroker` has no unit test — subscribe / broadcast / consumer-drop pruning.
- `DemodHandle` shutdown isn't exercised.
- WS `SignalingMessage` round-trip (especially the drag-select/SetCenter/SetViewport interaction that keeps biting) has no integration test.

### Priority triage

Top three I'd fix first, in order:
1. Session expiry (actually call `remove_expired`, or check TTL in `username()`).
2. Non-blocking broadcast/broker path so a slow WS client can't stall the USB callback.
3. The viewport race — the warning you already see in logs is from real users, not a synthetic bug.

Everything else is polish or defense-in-depth.
