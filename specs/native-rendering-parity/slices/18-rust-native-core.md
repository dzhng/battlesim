# 18 — Rust native core

**Contract:** native's CPU side is a Rust GDExtension, `native/godot-binding`, the native core.
- It owns the run loop, the capture and manifest readers, the report, the static-world selection (slice 15's walk) and every layer's instance writes.
- It writes instances straight through `RenderingServer`.
- GDScript shrinks to a thin entry scene that hands the command line to the core. Shaders stay as they are.

**Seam:**
- One crate with modules by concern: `capture`, `manifest`, `run`, `report`, `world` (buildings, scenery, props), `terrain`, `grass`, `environment`, `units`, `effects`. It is the same split as today's GDScript folders.
- Godot-facing classes stay few and thin.
- Data that crosses into shaders and compute (tables, uniforms, MultiMesh buffers) is written from Rust.
- Each GDScript module is deleted when its Rust replacement lands. There is no period where both own the same concept.

**Order:**
1. Static-world selection: in progress (slice 15's performance fix).
2. The runner, report, capture and manifest readers.
3. Terrain, grass and environment setup.
4. Units and effects move in slice 20, through the shared crate.

**Runnable artifact:** the same CLI and plates as today.

**Focused gates:**
- Plates are byte-identical to the GDScript build's at each step, within the noise floor.
- Counts gates (grass, instances, models, effects) unchanged.
- `tests/run.sh` becomes `cargo test` for logic plus the display tests for GPU paths, and no coverage is lost.
- Slice 12's performance bar: native at or below the browser on both scenes.
- CPU per frame reported at each step.
- **Less code.** Each GDScript module's Rust replacement is no larger than it, net of deleted tests; report added, deleted and net lines per language.

**Delegated:** module layout inside the crate, and godot-rust idioms.
