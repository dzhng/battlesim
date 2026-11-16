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
largest value at which the switch cannot be seen. Tests never read it. The
[census](../assets/census.md) bounds it: at the wide hold cars are 0.8 to
1.4 px and walls 0.5 to 0.9 px; at tactical range cars are 6 to 14 px and
walls 4 to 8 px. A value under 4 leaves the tactical view untouched.

## Steps

1. Author the far mesh; export, bake and check. Confirm the runtime diff is
   the car's bundle and nothing else.
2. Capture the review set below and pick `far_px`.
3. Measure by [the protocol](../README.md#measuring) at all three holds,
   against `staging`. Hold the close view for more frames than the others:
   slice 00 found three holds per batch cannot resolve 5% there. Add one
   whole-map hold as a diagnostic, since the player can zoom out that far.
4. Native: run the `counts` and `props` tests against the re-recorded count
   run. There is no native capture of the wide hold: the native client replays
   only saved parity reels, and this generated battle is not one
   ([decision 49](../decisions.md)).

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

**One named extra arm**, on its own pass, after walls so it covers both kinds:
*small off-view casters cast far.* A prop outside the view casts with its far
mesh when it is small enough that it would draw it, and with tier 3 otherwise.
Slice 03 counted 34,817 car casts at the wide hold that stay at tier 3 this
way. They come from cars more than their radius plus 3 m outside the frustum,
whose shadows cannot reach visible ground at this sun, so the arm should change
no pixel there. Adopt it only if it wins by the same tripwires and a fresh
critic finds no change, at this sun and at a low one.

The second arm the plan first named, tier-3 cars casting far, is dropped
unbuilt ([decision 48](../decisions.md)).

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
