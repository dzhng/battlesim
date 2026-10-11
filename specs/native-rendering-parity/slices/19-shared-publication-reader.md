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

**Focused gates:**
- **Equality.** The Rust reader matches the old TS decoder field for field on both fixture captures, before the TS decoder is deleted (the test from slice 13, reused).
- **The shipped browser is unchanged in what it draws.** Digest and replay unchanged, and the graphics test and battle scenes green. Its frame time and main-thread CPU on the graphics test (slice 07's harness) show no meaningful regression; a small measured cost is acceptable for less code (user, 2026-10-11).
- **Less code.** The slice reduces net lines across Rust, TypeScript, GDScript and tests (excluding comments and fixtures), reported as added, deleted and net per language. A slice that grows the total is reworked before it lands. This is the phase's point: one owner instead of an original, a port and a compare test.

- **Native.** Plates and counts unchanged.

**Delegated:** the typed observation's Rust shape.
