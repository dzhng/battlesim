# 18b — The presentation API

**Contract:** one small, documented API between the Rust side and every renderer. Designed and reviewed **before** slices 19, 20 and 20b expose anything to a renderer. A clean boundary is part of the phase's complexity goal (user, 2026-10-11): renderers operate over an API a reader can understand at a glance.

**Shape to design** (the proposal, refined by the design):
- **Calls,** the same set in the WASM and GDExtension bindings:
  - `load_world(map, catalogs)` → `World`;
  - `push_publication(words)`, called when a tick's publication arrives;
  - `frame(time, camera, viewport)` → `Frame`.

  That is a handful in total. No per-entity calls.
- **`World`:** static, given once. Terrain and grass tables, and instance lists by appearance and detail tier for buildings, scenery and props.
- **`Frame`:** per drawn frame, and only what changed. Instance transforms and tiers for movers and re-tiered chunks, joint palettes, effect instances by shape, cast lights, and the presentation clock.
- **Format.** Everything is flat and typed: views over memory the renderer uploads as-is. There are no game concepts a renderer must interpret, and no JS objects per entity.
- **The game UI is a separate surface.** The browser's HUD, readouts and fog display read the decoded observation through their own narrow call. Renderers never touch observations or game state.
- **One definition.** Types are defined once in Rust; both bindings are derived from that definition, not maintained by hand.

**Deliverable:**
- An API document in this file, written as the reference a renderer author reads, with each type's fields and units.
- A rendered diagram in `specs/native-rendering-parity/visualizations/presentation-api.html`.
- A list of today's TS and GDScript call sites that each call replaces.

**Gate:**
- The user reviews the API document. This is a contract, so it gets a real review, not a non-blocking one.
- Slices 19, 20 and 20b build only to this API. A change to it is a reviewed spec change.

**Delegated:** naming within the design, which the user may change at review.
