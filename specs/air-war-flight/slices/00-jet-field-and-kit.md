# 00 — Jet field and test kit

**Unlocks:** a place to watch jets fly, and a sim test kit to pin their behaviour. No rule changes.

## Seam
- **Test map:** a new authored map `fixtures/maps/air-jet/`.
  - About 3×3 km and flat.
  - A highrise of about 61 m, a village, an AA nest, and a road to each side's back edge.
  - `render_margin_m` of at least 2000.
- **Lab route:** each checkpoint in `apps/battle-lab/src/routes/airWar.tsx` and in `fixtures.json` gets a `map` field.
  - Helicopter checkpoints stay on `air`; jet checkpoints move to `air-jet`.
  - `FIELD` in `web/scenes/_air.mjs` becomes per map.
- **Scene helpers** in `web/scenes/_air.mjs`:
  - `trace(page, ticks, ids)` writes a PNG with a top-down track and a height-over-time strip, drawn from the observation;
  - `strip(...)` writes frames and a GIF (`ffmpeg`).
- **Sim test kit** `crates/sim/tests/common/jet.rs`:
  - `JetField::new(size)` builds a battle;
  - `.jet(kind, at, heading)` places a jet;
  - `Track::record(&mut Battle, ticks) -> Vec<Sample{pos, vel, agl, tick}>`.
- **Resolve the map's "confirm before coding" list,** and record each answer in `choices.md`:
  1. **Where the approach and departure effect hooks in.** What is published while the off-map delay runs (`publication.rs` `traversing`, own-side `PendingPurchase` in `contract/src/skirmish.rs`)? Can the presentation draw a jet model that has no unit behind it?
  2. **Which row an interceptor uses against a round.** Today `interception` names only `Missile` (`contract/src/weapons.rs`).
  3. **Whether any encounter relies on the four-sides spawn.** Encounters place their units directly, so the expectation is none.
  4. **Whether every aircraft family script builds** through `vehicle_export.run_disabled`, all 19.

## You see
`/labs/air-high` on `air-jet`, with today's hovering jet. Record it as the **before** GIF and trace. Every later slice compares against it.

## Tests
- A smoke test for the kit: a jet placed and recorded for 60 ticks yields 60 samples.
- The `air-high` scene loads on the new map.

## Firewall, delegation and what stays green
- **Firewall:** no sim rules, rows or behaviour.
- **Delegated:** the map layout, and the helper names.
- **Stays green:** every existing air test and scene.
