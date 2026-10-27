# 03: Water look data and variant picks

**Lane:** render. **Depends on:** 02.
**Visual variable:** none. Both river looks start equal to today's look.

## Contract

- **`WaterLook` records:** an array uploaded as a uniform. It replaces the
  water tuning that `presentation.terrain` writes into the shader text today
  (`terrainMaterial.ts`, `game.json` `presentation.terrain.water` and
  `shore`). Shader text stays the same for every battle.
- **Look rows:** they live in the biome data, with a weight per variant.
  Rivers get two: temperate green-grey and murky lowland (decision 9).
- **Water cells:** they become a `WaterCell` record `{corner, surface_z, side,
  look}`, built in `static_world/ground/water.rs`.
- **The export gains a river index** (`sim/src/world/export.rs` river stride).
  A junction cell takes the first river's look, as its surface already does.
- **One pick function:**
  `presentation::static_world::ground::water::look(biome_seed, feature) ->
  look index`. It is keyed like the per-cell seeding of tree stands
  (decision 27). Slice 13 reuses it for swamps.

## Seam tests

In `crates/presentation/tests/`:

- a river's look depends only on the biome seed and the river's index;
  adding a swamp or reordering features doesn't change it;
- a look with zero weight is never picked.

## Verification

1. The seam tests pass.
2. The `water` and `river` scenes are pixel-unchanged.
3. The shader-export and pipeline tests pass, with the tuning no longer in the
   shader text.
4. The native draw check passes.

No visual gate: nothing changes on screen.

## Native

The shader is shared, so this comes almost for free. Native uploads the
`WaterLook` array.

## Delegated to the implementer

- record layout;
- the maximum number of look rows.

## Must stay green

Battle digests and replays. All shots, exactly.
