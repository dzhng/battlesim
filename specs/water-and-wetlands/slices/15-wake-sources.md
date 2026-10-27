# 15: Wake sources

**Lane:** wet. **Depends on:** 13 (14 for puddles). **Visual variable:** none.
This slice only publishes data.

## Contract

- **Publication:** the effects-side mover record (`Shooter`,
  `crates/presentation/src/effects/publication.rs`) gains
  `wading: Vec<Wading { at, depth_m }>`.
  - It is filled by `EffectFeed::publication` from `StandingWater`, hull
    centres and member positions. `OwnUnit.members` already carries those
    positions.
  - Units carried in a vehicle are skipped. `riding()` moves from
    `units/interpolate.rs` to one place in the crate that both use.
- **Dust and wakes:** `EffectFrame::note_movers` extends the existing dust
  tracker. A hull in standing water gives wake impulses instead of dust
  (decision 8). Rounds landing in water splash instead of raising dust. There
  is no second per-unit tracker.
- **Impulses:** decided **per tick**, never from the pose driver's gait phase,
  which follows frame rate.
  - Rings come at smoke's fixed rate, with a keyed seed.
  - The V-wake comes at dust's spacing along the path.
  - The style is chosen by speed (decision 13).
  - Output: `WakeImpulse { xy, radius, strength, style, step }` records.
- **Visibility:** only units the side currently sees make impulses. Impulses
  already made fade on their own (decision 24).

## Seam tests

In `crates/presentation/tests/effect_behaviour.rs`, next to the dust tests:

- a hull moving in standing water makes a wake and no dust;
- a hidden unit makes no wake;
- a squad riding in an APC makes no wake, and the APC does;
- a still wader makes rings and a moving one drags a V;
- wakes don't depend on frame rate;
- a soldier crossing a puddle makes rings.

## The human sees

The `wetlands` lab has buttons for the moments the tests name: a hidden unit, a
seen unit, a rider, a still wader, a moving one, and a side switch. They show
where wake impulses are placed, as debug marks, before slice 16 draws any waves.

## Verification

1. The tests above pass.
2. `effect-vectors.json` changes only by the fields this slice adds.

## Native

Native consumes the records and ignores them until slice 16.

## Delegated to the implementer

- the strength constants;
- the speed threshold between rings and a V-wake.

## Must stay green

Battle digests and replays, and the existing dust behaviour on dry ground.
