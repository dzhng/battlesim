# 12: Swamps in map generation

**Lane:** sim. **Depends on:** 11. **Visual variable:** where swamps appear.

## Contract

- **Placement:** `crates/mapgen/src/layout` places swamps after towns and
  forests and before `verify`. They are drawn from their own random stream, so
  rivers, forests and towns don't move. Swamps respect `placed.reserved`, sit
  in flat ground near the river (or anywhere flat on maps with no river), and
  stay off settlements, buildings and bridge approaches (decision 5).
- **Presets:** each one in `fixtures/map-presets.json` gets a `swamp_chance`,
  like `river_chance`.
- **Versions:** bump `GENERATOR_VERSION` (`layout/mod.rs`) and the presets
  version.
- **Analysis:** `map_analysis::open_ground` and objectives refuse swamp ground.

## Seam tests

In `crates/mapgen/tests/layout.rs`, over the seed corpus:

- drawing swamps leaves every other feature unchanged;
- swamps keep clear of settlements, buildings and bridge approaches;
- a swamp lies near the river when there is one;
- swamp area is fair between the top and bottom halves;
- no objective lies in a swamp.

## The human sees

`bun run dev:maps` (the map workbench), stepping through seeds.

## Verification

1. The tests above pass.
2. Re-record `fixtures/parity/map-layout/` and the skirmish corpus.
3. Run `screenshot-critique` last, on a workbench contact sheet of 6 seeds, for
   placement only: does each swamp sit where a swamp would be?
4. Non-blocking checkpoint: show the sheet with `preview-shots`. Wait about 5
   minutes. If the user stays silent, decide, record the decision and continue.

## Pinned maps

The menu reel's market-town map is pinned at `layout-16`, stored as files, and
is **not** regenerated. That keeps the reel unchanged.

## Native

None.

## Delegated to the implementer

- the placement algorithm;
- the chance per preset;
- how big swamps can be.

## Must stay green

Pinned map files and their digests.
