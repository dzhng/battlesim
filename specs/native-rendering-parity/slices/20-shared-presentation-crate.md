# 20 — Shared presentation crate

**Contract:** the per-frame presentation derivations live once, in a Rust crate, `crates/presentation`. The browser runs them through WASM and native links them, and both clients use the same code.
- **Interpolation:** per-soldier blending.
- **The pose feed and pose driver**, including articulation, gait and the corpse limit.
- **Launches and the effect feed.**
- **The effect frame:** seeded bursts, dust, smoke and cast lights.
- **Cook-offs.**

The TypeScript originals and native's ports are deleted.

**Seam:**
- `crates/presentation` depends on `contract` and on slice 19's reader.
- It produces per-frame plain data: posed model instances with joint or part transforms, effect instances by shape, and cast lights.
- Each client draws that data. The browser keeps its TypeGPU passes and native its Godot passes.
- **Tuning data** (`presentation.effects`, pose tuning, unit facts) is read from the same fixture and catalog data both clients already load.
- **Seeded randomness:** Mulberry32 and `hashString` live in the crate. JavaScript's number-to-text behaviour, where the seeds depend on it, becomes a defined function that both sides use.

**Research first:**
- Per-frame call overhead and data marshalling from WASM on the main thread, for a battle's worth of units and particles.
- Whether outputs can be written straight into the browser's instance buffers (shared memory views).
- How the browser's renderer packages (`packages/battle-renderer`) consume them without a second copy.

Write the findings into this file before building.

**Order:** interpolation and poses first (gate: slice 14's pose vectors, then the model counts), then effects (gate: slice 17's effect vectors and counts).

**Focused gates:**
- **Parity with today.** Before any TS module is deleted, the Rust output equals today's TS output on the existing vector tests and on every plate's counts.
- **The shipped browser is unchanged.** Its plates are byte-identical to before, digest and replay unchanged, and graphics-test frame time and main-thread CPU no worse across three runs.
- **Native.** Plates and counts unchanged, and CPU per frame reported.

**Delegated:** the crate's internal structure.
