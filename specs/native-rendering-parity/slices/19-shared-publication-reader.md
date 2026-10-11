# 19 — One publication reader, in Rust

**Contract:** the simulation's publication is read by one Rust reader that sits beside its writer in the simulation crates. Native uses it directly, and the browser uses it through WASM. The TypeScript decoder (`web/src/battle/sim/observation.ts`) and native's GDScript port (`capture/observation.gd`) are both deleted.

**Seam:**
- A reader module next to `crates/sim/src/publication.rs` decodes the packed words into typed observations: own, identified, projectiles, blasts, corpses, known props, fallen bodies, crashes, guided, fog and ground runs.
- **Browser:** the reader is exposed through the existing WASM boundary (`crates/game-wasm`). Decoding happens where the words arrive (the main thread today), so the main thread gains a WASM instance or a reader module. That placement is researched first, below.
- **Native:** the core (slice 18) links the reader directly.

**Research first:**
- Where the browser decodes today, and what it costs per publication.
- Whether a main-thread WASM reader adds copies or latency.
- How views are handed to JS: zero-copy typed-array views over WASM memory are preferred.

Write the findings into this file before building.

**Order: web first.** Land the change in the browser first. Measure before and after with slice 07's harness on the graphics test and a medium battle: average FPS, 1% low, p95 and p99, main-thread CPU and GPU timestamps, three runs each. Only then switch native to the same code.

## Findings (2026-10-11)

All costs are CPU time per publication, the minimum of 15 passes over a whole capture's stream: node 24 for the TS decoder (Chrome's engine, V8), and a native arm64 release build for Rust (instructions retired, from `time -l`). Market-town is the control capture `2026-10-11T15-33-53-097Z`: 3,478 publications, a mean of 430 words (1.7 KB). Paris-corner has 1,291 publications.

**Where the browser decodes today.** On the main thread, in `client.ts`'s `receive`, through `ObservationDecoder.decode` (`observation.ts`), once per publication (30 Hz). That runs in the worker's message task, between frames, so the graphics test's per-frame `cpuMs` (slice 07's harness) never sees it. Its cost is measured on the stream instead:

| | market-town | paris-corner |
| --- | --- | --- |
| TS decoder, per publication | 110 µs | 41 µs |
| `JSON.parse` of the same view (33 KB mean, 46 KB max) | 57 µs | — |
| Rust reader: reconstruct, fog and ground only | 13 µs (0.22 M instructions) | — |
| Rust reader: whole view, as JSON values | 170 µs (3.07 M instructions) | 50 µs |
| `serde_json` text of that view | +67 µs | +12 µs |

At 30 Hz the TS decode is about 3.3 ms of main-thread time a second on market-town (0.3%). Own units dominate: 18 units are 25 KB of the 33 KB view.

**What crosses `postMessage` today.** The tick, digest, length and step time, plus one `ArrayBuffer` from a credited pool. The worker copies the record out of WASM memory into that buffer once and transfers it, so the words cross with no copy.

**A main-thread WASM reader.** The page already instantiates the module (`useStaticWorld`, map resolution), so the reader needs no new instance. The words cost one copy into WASM memory (1.7 KB, negligible), made from the transferred buffer. The way out is the cost. Any JS object graph built from WASM pays either `serde_json` text plus `JSON.parse` (67 + 57 µs before the decode), or a boundary crossing per field (`serde-wasm-bindgen`), which is worse. So a main-thread reader that hands the UI the whole view is slower than today's decoder.

**Zero-copy views.** These fit draw data (18b's `Frame`: flat instance arrays) and the fog bits and ground runs, which are already typed arrays. They don't fit the UI's observation, whose consumers read JS objects.

**Where the time goes in Rust.** Reconstruction, validation, fog and ground take 13 µs. A first view built as JSON values (one allocation per key) cost 412 µs per market-town publication in V8, 3× the TS decoder, so it was replaced by typed views that serde serializes directly.

## Web half (built 2026-10-11)

**Contract.** The page's own WASM module, `crates/presentation-wasm` (about 260 KB, against the battle module's 5.8 MB), is 18b's browser binding. It links the reader and no rules. The page side is `web/src/battle/sim/presentation.ts`.
- `new Presentation(layout)` takes the battle's `observation_layout`.
- `push_publication(words, arrived_s)` reads one record. It returns false for a stale record (from before a side change), and throws on a malformed one.
- `observation()` returns the view as JSON in `observationView.ts`'s shape. A static group is included only when it changed.
- `fog_bits()` returns the bits only when they changed. `ground_runs()` returns the runs as a `Float32Array`.
- `invalidate(side)` is the lab's side switch.

`ObservationDecoder` keeps its `decode`/`invalidate` contract over these calls, so every consumer and test reads the same `ObservationView` as before. `observation.ts` is deleted. Its types moved, unchanged, into `observationView.ts`.

**Equality** (before deleting the TS decoder):
- Deep equality (`toEqual`, `Object.is` on numbers) on every sample of both fixture captures.
- Static groups and fog bits are shared exactly where the TS decoder shared them.
- `isDeepStrictEqual` holds on every publication of both whole control streams (3,478 market-town, 1,291 paris-corner).

The one deliberate difference: JSON carries a non-finite number as null. The writer publishes NaN only in fields the view turns into null, so no real record differs. `groupDelivery`'s NaN-payload check now expects null; its baseline-XOR proof (1.5 recovered from the NaN's bits) stands.

**Cost per publication** (V8, min of 10 passes, market-town control stream): TS decoder 144 µs, Rust 180 µs. The Rust figure splits into decode 32, serialize 68, and parse with assembly 81. That is +36 µs, about 1 ms a second at 30 Hz.

**Graphics test** (`graphics-fps`, production build, headless, plugged in, before and after interleaved, three runs each). The per-run spread brackets every difference. Market-town's p95 sits on a display step (17.5 or 25 ms) on both sides.

| scene | | avg FPS (median; range) | 1% low | p95 / p99 ms | main-thread CPU ms (mean of runs) | GPU ms |
| --- | --- | --- | --- | --- | --- | --- |
| market-town | before | 73.1 (70.2–73.1) | 36.9 | 17.47 / 25.04 | 4.97 | 9.09 |
| market-town | after | 72.8 (70.8–74.0) | 37.3 | 23.36 / 25.32 | 4.76 | 9.24 |
| paris-corner | before | 104.4 (104.0–105.1) | 50.5 | 16.70 / 17.59 | 0.82 | 5.37 |
| paris-corner | after | 103.6 (93.0–104.8) | 55.5 | 16.73 / 17.76 | 1.01 | 5.58 |

Paris-corner's third "after" run was slower in GPU and CPU together (6.36 ms GPU, 93 FPS), which reads as contention, not decode.

**Battle** (the benchmark scene's short run: city contact on Metro Large, a heavy, main-thread-bound stress at 1920×1080; production build; interleaved, three runs each). Every difference sits inside the spread. The 1% low is the worst 9 of about 900 frames.

| | avg FPS (runs) | 1% low (runs) | p95 / p99 ms (median run) | main-thread CPU p50 / p95 ms (median run) | GPU ms (mean of runs) |
| --- | --- | --- | --- | --- | --- |
| before | 14.9, 15.1, 15.1 | 4.7, 4.6, 5.2 | 133.6 / 183.3 | 28.0 / 81.9 | 23.31 |
| after | 15.1, 15.0, 15.3 | 4.1, 4.7, 4.1 | 133.4 / 183.3 | 27.8 / 79.6 | 23.29 |

Every run of both scenes passed its checks (reel complete, timing comparable, no contention; the benchmark's phases, contact and camera keyframes). Evidence: `throwaway/perf/slice19-{before,after}/` and `throwaway/perf-runs.log`.

## Native half (built 2026-10-11)

The extension's `PublicationReader` (`native/godot-binding/src/publication_reader.rs`) wraps the same Rust reader. `open(layout)` returns false, with `error`, for a layout that isn't the writer's. `decode(words)` returns the sample's observation as Godot dictionaries and arrays of floats, the fog bits as a `PackedInt64Array` and the ground runs as a `PackedFloat32Array`, or null with `error`. An unchanged static group is the same `Array`, which `pose_feed.gd`'s `is_same` relies on. `presentation_capture.gd`'s `decode_samples` calls it, so the GDScript runner's consumers are unchanged. `observation.gd`, `publication_decode_test.gd` and the browser's fixture check (`nativePublicationDecode.test.ts`) are deleted. `crates/sim/tests/publication_reader.rs` stays as the reader's check against the fixtures. `native/godot/tests/run.sh` passes all 25 tests, including the learned-ground, pose-vector and capture tests that decode through the reader.

**Less code, whole slice** (code lines, excluding comments, blanks and fixtures, against `f53a07d05`):

| | added | deleted | net |
| --- | --- | --- | --- |
| Rust (reader, `GROUP_LAYOUTS`, presentation module, extension class, equality test) | 1,718 | 117 | +1,601 |
| TS (`observation.ts` out; `observationView.ts` and `presentation.ts` in) | 429 | 1,298 | −869 |
| TS tests | 81 | 151 | −70 |
| GDScript (`observation.gd` and its compare test out) | 3 | 954 | −951 |
| Total | 2,231 | 2,520 | **−289** |

Two duplicates of the reader remain for later: the Rust tests' own transport decoder (`tests/common/publication_codec.rs`, 140 lines) and the logical-record walker in `tests/publication.rs`.

**Focused gates:**
- **Equality.** The Rust reader matches the old TS decoder field for field on both fixture captures, before the TS decoder is deleted (the test from slice 13, reused).
- **The shipped browser is unchanged in what it draws.** Digest and replay unchanged, and the graphics test and battle scenes green. Its frame time and main-thread CPU on the graphics test (slice 07's harness) show no meaningful regression; a small measured cost is acceptable for less code (user, 2026-10-11).
- **Less code.** The slice reduces net lines across Rust, TypeScript, GDScript and tests (excluding comments and fixtures), reported as added, deleted and net per language. A slice that grows the total is reworked before it lands. This is the phase's point: one owner instead of an original, a port and a compare test.

- **Native.** Plates and counts unchanged.

**Delegated:** the typed observation's Rust shape.
