# 06 One table of shader constants

**Contract:** a number both the CPU and the GPU use has one owner. A number
only the GPU uses has a name in its WGSL.

**Seam:** `presentation::shader_constants`: one table of `(name, F32 | U32)`
entries. Each value stays in its concept's module; the table only lists them.
- `pipelines::builtin_source` (`renderer-core/src/pipelines.rs:414`) writes
  the prelude from it: `const N: f32 = …;` or `const N: u32 = …u;`.
- `records/typescript.rs` exports from the same table.
- Prelude names carry their owner's prefix (`FOG_`, `GROUND_`, …), as the
  `SWAY_`/`WAVE_` constants already do.

**Entries:**
- The five numbers with two owners: `grass.edge_m`, `forest_verge.feather`,
  `bank.ease_m`, `marks.clear_m`, `scars.depth_cap`. These are the Rust
  constants their tables already read (`ground/tables.rs`).
- `FOG_TILE_PX` and `FOG_RADIAL_BINS` as `u32`. They size arrays and
  `@workgroup_size` (`passes_fog.wgsl:206-207,331,363`). `sight.rs:96-106`'s
  accepted ranges and the fog uniform's two fields go.
- The other duplicates: `CASCADES`, `CAST_LIGHTS_MAX`, `FOLIAGE_STEP` and its
  255, `WHOLE_TEXTURE_WIDTH`, `HORIZON_SECTORS`, `BUILD_WORKGROUP`.
- The 187 numbers 05 deleted from data become WGSL-local `const`s, named for
  what they do. Keep each literal's text exactly.

**Run / see:** nothing changes.

**Verify:**
- **Substitute-back diff:** take the composed source for every pipeline, put
  each new name back to its literal text, and diff against 01's dump. It must
  be empty.
- Native plates byte-identical to 01's.
- **Browser compile:** run one scene (e.g. `bun run --cwd web scene --
  unit-roster`). It proves the browser's compiler accepts the integer prelude
  constants; naga's acceptance is already settled.
- Tests: `cargo test -p renderer-core --test pipeline_abi`,
  `native-client --test pipelines`. Then
  `UPDATE_RECORDS=1 cargo test -p presentation records_are_current`, and
  rerun it unflagged.
- `sight.rs:1378` (`radial_bins: 2, tile_px: 1`) and `sight.rs:1689` (refuses
  17) test numbers that are now constants. Rewrite them to test what the fog
  does, not the removed range check.

**Moves:** generated `records.gen.ts` and the fog uniform layout. No hash
moves.

**Depends on:** [05](05-typed-readers.md).

**Delegated:** table syntax; the WGSL constant names.

**Would change this slice:** a substitute-back diff that isn't empty, or a
moved plate. Find the literal; never re-bless.
