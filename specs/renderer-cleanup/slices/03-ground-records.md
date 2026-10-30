# 03 · Ground records: terrain, grass, scars and their masks

**Contract.** Presentation fills typed ground records instead of appending
words by hand, and every ground length and mask in WGSL is a named constant.

## Seam

- Records in `presentation::records`:
  - `TerrainParams { roads: [RoadLook; ROAD_LOOKS_MAX], shore_tuning: ShoreTuning, water_looks: [WaterLook; WATER_LOOKS_MAX] }`
    (packed today in `static_world/ground/tables.rs`; `ROAD_LOOKS_MAX` is
    `AREA_KINDS.len()`, `biome.rs:603`; `WATER_LOOKS_MAX`, `biome/water.rs:26`);
  - `GrassParams` (`ground/grass.rs:90-170`, `:530-580`);
  - `ScarParams` (`renderer-core/src/scars.rs:14` `WORDS = 52`; the flag word,
    `grid.w`, at `:128` and `:306-308`).
- Packers keep writing words but derive sizes and offsets from the layout
  (`size / 4`, field offsets), so uploads stay byte-identical by construction.
  `ground/tests.rs:384-397` uses the layout's offsets instead of `24 + tag * 56`.
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
