# 03 · Ground records: terrain, grass, scars and their masks

**Contract.** Presentation fills typed ground records instead of appending
words by hand, and every ground length and mask in WGSL is a named constant.

## Seam

- Records in `presentation::records`:
  - `TerrainParams { roads: [RoadLook; ROAD_LOOKS_MAX], shore_tuning: ShoreTuning, water_looks: [WaterLook; WATER_LOOKS_MAX] }`
    (filled in `static_world/ground/tables.rs`; the bounds come from the road
    kinds and water biome);
  - `GrassParams` (`ground/grass.rs`), including typed per-frame updates;
  - `ScarParams` (presentation's scar cache and `renderer-core/src/scars.rs`;
    `grid.w` is the enable flag).
- Packers fill the typed records, then cast them with bytemuck to the uploaded
  words. Float arithmetic stays f64 until the final f32 conversion; integer
  fields stay u32. Offsets only delimit partial record publications or GPU
  writes; variable-length tables retain append packing. Ground tests read the
  layout's offsets instead of maintaining their own record strides.
- `shader_constants` exports `WATER_LOOKS_MAX`, `ROAD_LOOKS_MAX`,
  `SCAR_KEY_MASK` (`scars/mod.rs:27`) and `GROUND_RECORD_MASK`
  (`ground/field.rs:28-29`): two concepts, two names (map unknown 4). The WGSL
  uses them in place of `8`, `4` and the seven `536870911u`, and names the flag
  bits (`2147483648u`, `536870912u`, `>> 30u`) from their owners.
- Fix the stale doc at `tables.rs:22` ("eight vec4s"; there are ten).

## Verify

- `pipeline_abi` reflects the three records; `cargo test -p presentation`.
- A constant moved into the prelude can change float rounding (map unknown 1);
  the assembled-shader diff shows exactly which pipelines changed.
- All three reels byte-identical; GPU pixel tests (scars); browser scenes
  `ground`, `river`, `water`.

**Delegated:** record field names where WGSL members need none changed.
**Stays green:** every ground upload byte for byte.
