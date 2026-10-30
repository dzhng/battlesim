# 13 · renderer-core's tests in renderer-core

**Contract.** `cargo test -p renderer-core` runs the renderer's own GPU tests;
native keeps no re-export shims.

## Seam

- Probe first: add dev-dependencies on `wgpu` with its default (native) backends
  and on `pollster`, plus `tests/support/mod.rs` with the device helper from
  `native/client/src/gpu.rs`. Move `pipelines.rs` (22 lines) and run
  `cargo test -p renderer-core`. Confirm `bun run build:wasm` is unaffected
  (dev-dependencies stay out of the wasm build).
- Then move `native/client/tests/{high_air,impostors,xray,probes,marks,fog_views,frame_views,scar_visibility,model_geometry,water_sampling}.rs`.
  `fixtures/presentation.json` resolves from the new path unchanged.
  `model_geometry` opens the runtime the way `tests/grass_assets.rs:18` does
  (map unknown 10).
- Delete the shims: `native/client/src/lib.rs:11`'s re-export (native's own
  users of `models` and `scenery`, `tests/models.rs:8` and `src/capture.rs:30`,
  import them from renderer-core directly), `pipelines.rs`, and `bundle.rs`'s
  re-export line. `bundle.rs`'s filesystem `Library` is production code and
  stays.

## Verify

- `cargo test -p renderer-core` under the GPU lock: 20 tests today, about 39
  after.
- `cargo test -p native-client` still passes; `bun run build:wasm`.

**Delegated:** helper names; one shared readback helper if several tests'
copies are identical.
**Stays green:** every moved test, unchanged in what it asserts.
