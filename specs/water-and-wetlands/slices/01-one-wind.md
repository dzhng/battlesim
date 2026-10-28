# 01: One wind

**Lane:** render, parallel with slices 02 and 10. **Depends on:** 00.
**Visual variable:** the drift heading of smoke and dust.
**Status:** done (2026-10-28).

## Contract

Rust presentation resolves **one `Wind` record** from the biome, and both
clients upload it in the `Environment` uniform, after the haze (`Aerial`).

- **The record** (`crates/presentation/src/records.rs`, generated into
  `records.gen.ts`), 32 bytes:
  - `toward`: `vec3f`, the level unit vector it blows toward;
  - `drift_mps`: how fast it carries smoke and dust;
  - `gust_mps`: how fast a gust front rolls downwind;
  - `gust_per_m`: one over the gust fronts' spacing;
  - two words of padding.

  Gust speed and drift speed are separate quantities (decision 26). The
  record holds no clock: grass sways on the camera's `time`, and smoke on the
  presentation clock.
- **Source data:** the biome's top-level `wind` block, `{ heading_deg,
  drift_mps, gust_m, gust_mps }` (`fixtures/biomes/summer.json`). Grass keeps
  only how it answers, in `grass.sway` (`lean`, `gust`, `flutter`,
  `flutter_hz`), and the shader's response tuning in `game.json` is
  `presentation.grass.sway`.
- **Owner:** `presentation::wind` (`WindSettings`: the block, its checks and
  its record).
- **Readers:**
  - grass's vertex stage reads `environmentWind()`
    (`world/environment.ts`). The grass pipelines now bind the environment,
    so their groups are camera, grass, environment, fog;
  - the effects drift smoke and dust on the record's direction
    (`EffectFrame::build` takes the biome's block, and none means calm air).
- **Upload:** the browser's `battleFrame.ts` hands the presented ground's
  record (`ground_wind`) to the environment frame (`setWind`). Native passes
  it to `Environment::new` with the light, after resolving the ground.
- **Deleted:** `presentation.effects.wind_mps`, the biome's `grass.wind`, and
  the wind words in `GrassParams` (one `vec4f` fewer). The old sources and
  the new one never coexist.
- **Heading:** the grass's 35°. Smoke and dust keep their 0.67 m/s and turn
  from 26.6° to 35°, which is 8.4°.
- **Out of scope:** the sound's wind (`synth.rs`). It stays separate.

## What changed from the plan

- **No clock in the record.** The camera already carries one, and a copy
  would be written every frame for nothing ([`choices.md`](../choices.md)).
- **No ground means calm air.** Before, every presentation drifted on the
  effects' fixed wind.
- **Native smoke plates.** None are committed. The paris-corner run fixture
  holds effect counts only, and those don't move.
- **Room in over-budget files.** `effects/frame.rs` was split under the size
  budget in its own commit first. `presentation.rs` made room by moving
  `aircraft_shadow` to `shadow.rs`.

## Seam test

`crates/presentation/tests/effect_behaviour.rs`,
`smoke_and_dust_drift_the_way_the_grass_leans_whatever_the_heading`. At six
headings, a wreck's smoke and a moving hull's dust are drawn in the wind and
in still air. Every puff moves along the record's `toward`, the direction
grass leans, with no sideways or vertical part, and most move downwind. It
went red when the drift was pinned to the old fixed wind, and again when the
drift was turned 8° off the grass.

## Verification (done)

1. The seam test passes. `cargo test -p presentation` passes except the
   known `poses.rs` facing drift, which also fails on `staging`.
2. Re-recorded as one decision: `effect-vectors.json` with its new bless path
   (`BLESS_PARITY=1 cargo test -p presentation --test effects`), and
   `fixtures/parity/exact-numbers.json` (`BLESS_PARITY=1`), which moved only
   by the renamed and deleted keys. Its digests are unchanged, and the sim
   suite passes. `grass-vectors.json` changed only its input block; its
   recorded words are unchanged.
3. The shader export is current, `native/client/tests/pipelines.rs` builds
   every pipeline, and the water scene passes on both clients.
4. Grass is unchanged. In the street battle's frozen frames, before against
   after differs only on the smoke plumes. The scattered grass pixels that
   differ are the same flicker two runs of one build show: grass draws in a
   different order from run to run. The water reel's plates differ only by
   that flicker and by rifle smoke. The menu reel can't judge this, because
   its plate timing drifts between runs of the same build.
5. `compare-screenshots` on the craters frame's smoke: only the plume moved.
   It leans further downwind, and the ground, craters, wreck and fire are
   the same. An unprimed `screenshot-critique` of the two crops found
   differences only inside the plume and no defect in one image only. It
   also judged the smoke too tan and dust-like, with no dark core. That holds
   before and after, and belongs to the smoke's look, not to the wind.

## Native

Ported: the record rides in native's environment uniform, its grass binds by
layout, and its smoke comes from the shared presentation. No gap.

## Must stay green

All battle digests and replays (this is presentation only), and grass shots.
