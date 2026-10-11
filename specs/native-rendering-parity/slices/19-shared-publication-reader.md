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

**Where the time goes in Rust.** Reconstruction, validation, fog and ground take 13 µs. Building a per-key JSON value view (one allocation per key) is 93% of the cost. That's fine for native's startup decode (0.6 s for the whole stream, against GDScript's 4.1 s) and for tests. It's too slow for the browser's main thread. Under 18b, therefore:
- `push_publication` should decode only (13 µs).
- The presentation crate should read the reconstructed rows through the writer's field constants, with no per-entity view.
- `observation()` should build the UI's view lazily, and only the part the UI reads. Units' members and poses go to `Frame`, not to the UI.

**Built so far** (18b gates the surfaces):
- `crates/sim/src/publication/reader.rs` holds the reader: one ordered stream, committed only on success.
- `layout_json` and the reader now share one group table (`GROUP_LAYOUTS`), and the reader refuses any other layout.
- `crates/sim/tests/publication_reader.rs` checks that every sample of both fixture captures equals the TS decoder's committed decode, at float32 precision. A one-field mutation fails it.

The reader needs only the layout tables, `serde_json` and std. If 18b's separate `presentation-wasm` module holds, reader and tables move out of `sim` together, with no rewrite.

**Less code, so far** (code lines, excluding comments, blanks and fixtures):

| | added | deleted | net |
| --- | --- | --- | --- |
| Rust (reader, `GROUP_LAYOUTS`, equality test) | 1,332 | 95 | +1,237 |
| TS, GDScript | 0 | 0 | 0 |
| Due when the surfaces land | | `observation.ts` 1,197, `observation.gd` 836, `nativePublicationDecode.test.ts` 77, `publication_decode_test.gd` 84 | −2,194 |

That projects about −960 before the binding glue. Two more duplicates could go as well: the Rust tests' own transport decoder (`tests/common/publication_codec.rs`, 140 lines) and the logical-record walker in `tests/publication.rs`.

**Focused gates:**
- **Equality.** The Rust reader matches the old TS decoder field for field on both fixture captures, before the TS decoder is deleted (the test from slice 13, reused).
- **The shipped browser is unchanged in what it draws.** Digest and replay unchanged, and the graphics test and battle scenes green. Its frame time and main-thread CPU on the graphics test (slice 07's harness) show no meaningful regression; a small measured cost is acceptable for less code (user, 2026-10-11).
- **Less code.** The slice reduces net lines across Rust, TypeScript, GDScript and tests (excluding comments and fixtures), reported as added, deleted and net per language. A slice that grows the total is reworked before it lands. This is the phase's point: one owner instead of an original, a port and a compare test.

- **Native.** Plates and counts unchanged.

**Delegated:** the typed observation's Rust shape.
