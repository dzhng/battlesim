# 15 · V-wakes read on screen, or `kind` goes

**Contract.** A fast mover in deep water trails a V the player can see, as
water-and-wetlands decision 13 promised ("rings, plus a V-wake once the unit
moves"). The wave splat's `kind` is either used or deleted.

## Today

- Fast movers in deep enough water drop a trail of V-wake splats along their
  path (`crates/presentation/src/effects/frame/wake.rs:36-50`); slow movers pulse
  rings in place (`:52-69`). The wave field propagates what's dropped
  (`crates/renderer-core/src/water.rs`), so a V may already form.
- `water.wgsl:173-179` splats every splat as a ring and never reads `kind`.

## Steps

1. Look first: capture the browser `water` scene with a fast mover in deep
   water. Native plates can't judge this (native's wave field runs fewer steps,
   `specs/water-native/README.md`; map unknown 11).
2. **If a V already reads**, delete `kind` (`records.rs:408-417`, `wake.rs:117`,
   the WGSL member, `tests/effect_behaviour.rs:519-524`,
   `tests/wake_sources.rs:255`). The record keeps its size with padding, so all
   three reels stay byte-identical.
3. **If not**, `waveStep` shapes V-wake splats from `kind` (and from the trail's
   direction, if a heading must be added to the splat, within the existing wake
   budgets). Add a failing directional-wake observation first.
4. Fix the stale `specs/done/water-and-wetlands/README.md:559` ("native draws no
   wakes").

## Verify

- **Slice variable:** the wake shape behind a fast mover. **Crop:** the water
  around and behind the mover. Out of scope: water colour, shore, rings around
  slow or standing units.
- Use `compare-screenshots` on before vs after crops, then an unprimed
  `screenshot-critique` as the last check.
- **Sign-off (the user's call, map Q3).** Open the before and after shots with
  `preview-shots` and give the user about 5 minutes. Silence doesn't block the
  rest of the spec: keep the change on its own commit, finish the closeout, and
  don't merge a changed picture until the user has signed off. Close the opened
  shots.
- Slow and shallow rings, fog isolation and wake budgets stay green
  (`wave_field`, `wake_sources`).

**Delegated:** how the V is shaped and encoded, within the wake budgets.
