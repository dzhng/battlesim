# 03: Parked cars carry a far mesh

The first shipped content, and the slice that proves the win.

## What this unlocks

Parked cars draw and cast a far mesh below `far_px`. The wide view submits
fewer triangles and takes less GPU time, and no car looks different at
tactical range or closer.

## Seam

Content and data only; no new code path.

- `packages/scene-assets/blender/street_car.py` adds the `_LOD4` parts. The
  wreck (`--wreck`) stays at four tiers.
- `packages/scene-assets/src/scenery.ts`: `parked_car.tier_triangles` gains
  its fifth entry.
- `fixtures/presentation.json`: `models.far_px` gets its shipped value.

**The far mesh keeps:** the car's outer silhouette (length, width, roofline),
its paint slot so each car keeps its own colour, and its contact with the
ground. **It drops** only what is under a pixel at the threshold: wheels and
arches, glass as separate geometry, bevels, buried and bottom faces.

**The threshold** is chosen from the zoom crops, not from the timing: the
largest value at which the switch cannot be seen. Tests never read it.

## Steps

1. Author the far mesh; export, bake and check. Confirm the runtime diff is
   the car's bundle and nothing else.
2. Capture the review set below and pick `far_px`.
3. Measure by [the protocol](../README.md#measuring) at all three holds,
   against `staging`.
4. Native: run the `counts` and `props` tests against the re-recorded count
   run, and take one native capture of the wide hold whose submitted counts
   match the browser's.

## What the user can see

- The car's review sheet with five tiers.
- The wide hold, before and after.
- A zoom path from 65 m to the wide hold at the receipt's heading and pitch,
  with two stops either side of the threshold, in and back out.

## Checks

**Slice variable:** a parked car's presence, colour and ground shadow at a few
pixels. **Crop:** car-dense street blocks at the wide hold and at the two
threshold stops, each with its ground shadow. **Out of scope:** the fog hatch,
buildings, trees, units.

1. Counts: equal instances in both arms, and the expected triangle drop, in
   the view pass and the cast pass separately.
2. Timing: the tripwires in the protocol. No measurable wide-view win means
   the far mesh does not ship; record the result and reslice.
3. A car outside the view whose shadow lands on visible ground: same caster,
   same cascades, same shadow on the ground as `staging`.
4. No flicker: crossing the threshold in and out along the zoom path shows no
   chatter at a held camera. There is no hysteresis; if a pop is visible, stop
   and write a slice for it before adding state.
5. Digest, replay and the side's publication equal `staging`.
6. [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
   on the crops, candidate against `staging`.
7. A fresh, unprimed
   [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
   as the last check.
8. [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md): open the
   paired crops for the user, wait about five minutes, then decide on the
   evidence, record it in [the ledger](../decisions.md), and close the shots.

**One named extra arm**, only after the above passes: tier-3 cars also cast
with the far mesh. Measure it and take the tactical-range shadow crop. Adopt
it only if it wins by the same tripwires and the crop shows no change a fresh
critic can find. Record the verdict either way.

## Delegated

- The far mesh's exact shape and triangle count.
- At most two mesh variants and two threshold values before a verdict is
  written.

## Must stay green

Tiers 0 to 3 of the car, byte for byte; the wreck; every global check in the
README. The menu reel follows the game: record it again if it moves, never
block on it.

## Feedback that would change this slice

- "Cars look like slabs" or "they lose their heading": keep more of the cabin,
  or lower the threshold. Never hide it with fog or scale.
- "Cars pop on zoom": lower the threshold first; hysteresis is its own slice.
