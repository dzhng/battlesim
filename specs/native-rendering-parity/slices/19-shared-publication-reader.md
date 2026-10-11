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
- **The shipped browser is unchanged.** Digest and replay unchanged, the graphics test and battle scenes green, and the browser's frame time and main-thread CPU no worse on the graphics test (slice 07's harness).
- **Native.** Plates and counts unchanged.

**Delegated:** the typed observation's Rust shape.
