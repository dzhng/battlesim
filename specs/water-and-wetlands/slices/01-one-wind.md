# 01: One wind

**Lane:** render, parallel with slices 02 and 10. **Depends on:** 00.
**Visual variable:** the drift heading of smoke and dust.

## Contract

Rust presentation resolves **one `Wind` record** and uploads it the way the
light records are uploaded.

- **Fields:**
  - heading;
  - drift speed (smoke and dust);
  - gust speed and spacing (grass);
  - the clock.

  Gust speed and drift speed are separate quantities (decision 26).
- **Source data:** the biome's `wind` block, which holds heading, drift and
  gusts. Grass keeps only how it responds (lean, flutter) under its own data.
- **Owner:** a new `presentation::wind` module. The record is appended to the
  `Environment` uniform (`packages/battle-renderer/src/world/environment.ts`),
  which every material already binds, so no bind group gains an entry.
- **Deleted:** `presentation.effects.wind_mps` (`effects/tables.rs`), the
  biome's `grass.wind.heading_deg`, and the wind words in the grass params.
  The old sources and the new one never coexist.
- **Heading:** the grass's 35°. Smoke and dust turn by about 8° (decision 26).
- **Out of scope:** the sound's wind (`synth.rs`). It stays separate.

## Seam test

`crates/presentation/tests/effect_behaviour.rs` gets a test that smoke and dust
drift along the direction grass leans, for any heading. That tests the
mechanic, not a value.

## The human sees

`/labs/wetlands` and the menu reel's smoke shot. Grass doesn't change; smoke
drifts with it.

## Verification

1. The seam test passes.
2. Grass pixels are unchanged in the `water` scene and the menu reel's grass
   shots.
3. Re-record, as one named decision: `crates/presentation/tests/fixtures/grass-vectors.json`,
   `effect-vectors.json`, `fixtures/parity/exact-numbers.json` (it is compared
   as text), and native smoke plates.
4. Run `compare-screenshots` on a smoke crop, before against after. The verdict
   is that only the heading changed.
5. Run `screenshot-critique` last, on the smoke crop.

## Native

Must be ported in this slice. The record is shared, and grass's layout
changes.

## Delegated to the implementer

- record packing and alignment;
- where in the biome data the source block sits.

## Must stay green

All battle digests and replays (this is presentation only), and grass shots.
