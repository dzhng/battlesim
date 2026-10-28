# 05: The soft shore

**Lane:** render. **Depends on:** 04. **Visual variable:** shoreline softness
and placement.
**Status:** done (2026-10-28).

## Contract

- The water edge fades by optical depth, with a fade distance floored by camera
  distance, so it never aliases (RedGPU `lakeFragment.wgsl:373-375, 630`).
- It replaces the analytic shore bands in `terrainMaterial.ts` (`waterSurface`
  and the shore and bed bands). Those are deleted. The same fade gives swamp
  and puddle edges whatever their shape.
- Water geometry still comes from presentation's water cells.

## What changed from the plan

- The water's analytic edge is gone. The bank's wet silt and bare earth, and
  the bed's colour under the water, stay as the ground's look
  ([choices](../choices.md)).
- The fade's three-pixel floor is measured at the pixel's centre, not from
  the centroid `world`, and its slant floor is 0.35, not RedGPU's 0.12.
- `shore_fade_m` (0.25 m) is shared tuning in `presentation.terrain.water`.

## Verification (done)

1. The river scene's checks of shore placement and waterline flicker
   passed, where the slice expected them to fail. Re-judged, they still test
   what the player sees; none was changed.
2. `compare-screenshots`, on bank crops at the close, tactical and low
   framings, against slice 04's plates and `../assets/redgpu-wading-shore.jpg`.
   - Close: before, the water met the sand at a short, hard fade. Now the
     water deepens across a band of sandy shallows.
   - Tactical: before, the dark water stood hard against a pale lip. Now it
     shades softly into the bank.
   - Low: before, a crisp cut-out edge under a pale rim. Now there is no hard
     line, and the bed shows through the shallows, as in the reference.
   Verdict: less wrong.
3. An unprimed `screenshot-critique` of the bank crops found three things,
   and both fixes were checked again:
   - faint grid lines in the shallows (the water's triangle edges, through
     the centroid footprint), fixed;
   - a milky band at the low view, narrowed;
   - streaks at full strength in the shallows, and no bed texture: slices 06
     and 07.
4. The water scene passes; native's river matches the browser's.
5. A selected squad's ring crossing from the bank into the shallows keeps
   one strength: the mark is its own layer over the fading water.
6. `fixtures/parity/exact-numbers.json` was re-recorded for the new
   `shore_fade_m` key, which it embeds with the game fixture. Its digest rows
   are unchanged.
7. `web/scenes/baselines/frame/river.png` and `geometry.png` were
   re-recorded: their shores softened, and nothing changed off the water.

## Native

Free (shared shader).

## Delegated to the implementer

The camera-distance floor constant.

## Must stay green

Digests and replays, and the slice 04 water crop away from the bank.
