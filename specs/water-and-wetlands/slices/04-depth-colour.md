# 04: Depth colour

**Lane:** render. **Depends on:** 03.
**Visual variable:** colour against depth, in the two river looks.
**Status:** done (2026-10-28).

## Contract

- **The composite** (`frame/waterPass.ts`): the bed under the water at its
  pixel (`sceneColour`, a copy of `lit` taken after the world pass) times
  `exp(-e·d)` per channel, where `d` is the distance along the view down to
  the world's depth (capped at 50 m), plus the look's body colour, lit, in
  place of what the water takes, plus the reflection whole. The body turns
  from the look's edge colour to its channel colour as less light survives.
  The water is opaque over its bed; its alpha is only its edge feather.
  The air between the water and the eye goes on once: `shade` is
  `air(surfaceLight(...))` in the environment, both exported, and the
  water composes its light before the air.
- **Look data** (`biome.water.rivers`): each look has `palette`,
  `extinction_per_m: [r, g, b]` and `streak`. `opacity` and `shallows_m`
  are gone. Summer's temperate green-grey and murky lowland now differ.
- **What the water reads:** the fog layout gains `depth` and `sceneColour`.
  Only `FogGroups.water` holds the real ones; every group the world pass
  binds holds a 1×1 placeholder. `FrameTargets.sceneColour` is new, and
  `lit` gains `COPY_SRC`. Native mirrors all of it.
- **Paint:** the bed is unpainted under the water, and the water lays the
  mark over its composite whole.
- **The native draw check** (the water scene): the browser's river stands
  at least ΔE 10 from its bank, and native's lies within ΔE 3 of the
  browser's. `WATER_LOOK=<row>` films one look; native ignores the pin.

## What changed from the plan

- The extinction is per channel in each look, not RedGPU's fixed ratios:
  those turned our bed a saturated green and teal, against decision 9.
- Slice 00's "bluer than its bank" check was re-judged, as its entry asked:
  a green-grey or brown river is not bluer than grass.
- Water's fog coverage keeps the water's own sight term, now over an
  opaque surface: the water is the thing seen there.

## Verification (done)

1. `compare-screenshots`, on the water at the close and tactical framings,
   against slice 00's plates and `../assets/redgpu-depth-colour.jpg`. Lightness
   and hue (CIELAB) of the bands in from the edge:

   | | 0 to 2 m | 2 to 5 m | past 8 m |
   | --- | --- | --- | --- |
   | before (one look) | L* 41, b* −2 | L* 38, b* −5 | L* 38, b* −5 |
   | temperate | L* 47, a* −3, b* 17 | L* 38, a* −7, b* 13 | L* 27, a* −7, b* 7 |
   | murky lowland | L* 44, a* −2, b* 20 | L* 33, a* −1, b* 18 | L* 28, a* 2, b* 17 |

   Before, the water was one flat band. Now it shows its bed in the shallows
   and takes its body's colour with depth, as the reference does. Verdict:
   less wrong.
2. Paint shows once: a selected squad's ring at the river's edge crosses onto
   the water as one line, with no second or brightened copy (a throwaway
   probe in the river lab).
3. `web/scenes/baselines/frame/river.png` and `geometry.png` were
   re-recorded. Both are taken on Low, which keeps depth colour (decision 18),
   and they changed only on their water.
4. An unprimed `screenshot-critique` of both looks, on an earlier tuning,
   found the temperate look a saturated bottle-green and too dark from
   above, and the murky one chocolate-dark under a chartreuse rim. The last
   tuning answers those. Its other notes belong to later slices:
   - the shallows as a gradient over a featureless bed (07);
   - no waterline, and a pale rim at a low angle (05);
   - streaky ripples (06);
   - low-angle teal from the sky's reflection (09).
5. The water and river scenes pass. Native's river matches the browser's to
   ΔE 0.0 on every shot.
6. The checkpoint: both looks, before against after, the reference and the
   paint crop were shown with `preview-shots`. No feedback came within about
   three minutes, so the tuning stands.

## Native

Ported: the scene colour copy, the fog groups' views and the water's group.

## Must stay green

Battle digests and replays, and everything outside the water mask.
