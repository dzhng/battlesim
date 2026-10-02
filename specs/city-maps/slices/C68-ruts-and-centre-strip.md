# C68: ruts and centre strip

**Depends on:** C67 (first in the ground cut order). **Kind:** slice.

## Question
Do ruts and a dirt-track centre strip add close detail without a ghost outline at 65 m?

## Contract it unlocks
- Two noise-broken tracks inside the core, sunk by a **normal tilt of ≤15°** (the terrain is never displaced).
- They fade to the core's mean below about 2 px per rut, using the same footprint fade as the plot rows (`terrainMaterial.ts:445-450`).
- On dirt tracks below a width threshold, grass grows in a centre strip, keyed from the field.
- SG3's verdict can reduce this to roughness and normal only, or cut it.

## API seam
`terrainMaterial.ts` road term, `grassPass.ts` growth.

## What the human can run or see
`bend-25`, then `bend-65/120/250` (to show the fade), and the lab's `track-25/65`.

## Verification
- At `bend-250` the core's variance is within 1% of C67's; at `bend-25` the ruts are measurably present.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the road core only; critique asks "does the road read as an outline?" at 65 m**) against **`../assets/reference/ground/forest-road-rocks.jpg`, `forest-road-summer.jpg` and C67's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Shoulder, grass outside the core.


## Delegated to the implementer
Rut spacing and darkness (bounded by the shadow floor), the centre-strip threshold. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C67's shoulder.

## Feedback that would change this slice
Ruts that dominate the road read change rut coverage/depth; road width and material are frozen inputs.

## Outcome

**Built for the dirt track; cut on the country road** ([SG3](SG3-road-wear-read.md#outcome)). A dirt track up to 5 m wide has a pair of wheel ruts and a strip of grass between them. The gravel road's `ruts` row is empty: every shape of rut tried on it read as pinstripes.

**The seam.**
- **`groundPaved` returns `GroundPaved`:** the distance inside each kind's paving as before, and the lane the point is in: for the stroke it lies deepest in, the unit vector away from its centreline, the distance from it, its half width and its kind. A polygon has no centreline, so a town's streets have no lanes.
- **`groundRuts(xy, footprint, paved)`** returns the slope a rut's side gives the shading normal and a shade for the albedo. Ruts run at `ruts.offsets_m` either side of the centreline, `width_m` wide, where the stroke is wide enough to hold them. The tilt is at most `tilt_deg` (5° on the track; the biome refuses more than 15°), and the terrain is never moved. The shade darkens the rut and lightens the rest by the ruts' share of the road, so a faded rut leaves exactly the surface without ruts. They are whole while a pixel is under 0.2 of a rut's width and gone at 0.4, and come and go along the track by noise.
- **`groundStrip`** is the centre strip: `centre_strip.half_width_m` either side of the centreline on strokes no wider than `max_road_width_m`, its edge wandering, never broken along its length, fading with the ruts. The ground there goes 60% to the verge's grass colour, and `groundShoulder` reports less wear there, so the grass build (which now leaves a road bare by that wear, not by a margin of its own) grows clumps on it.
- **The terrain fragment** adds the ruts' slope to the shading normal beside the bank's and the scars'.

**Measured** (the `ground` scene's road checks; the river lab's 4 m track, bare ground):
- Ruts against the surface beside them: 4.7% darker at `track-25`, 0.2% at `track-250` (a new station).
- The track's middle is greener than its ruts at `track-25`: green over red 0.90 against 0.82. At `track-250` they read 0.86 and 0.84: the strip is gone.
- The gravel road's core is byte-identical to C67's at `bend-65` and `bend-250` (it has no ruts), which is the slice's "variance within 1% at `bend-250`" with nothing to spare or to argue.
- Frame cost, with C66 and C67: the roads' wear on against off, four interleaved pairs, on the build that still had ruts on the gravel road: −0.03 ms at village `bend-65` (pairs −0.03, −0.15, +0.02, −0.12) and −0.04 ms at `town-65` (+0.39, −0.13, −0.04, −0.14) on a 2.8 ms frame. Under what four pairs resolve; one run.
- The surface, field, forest and rig probes of the `ground` scene pass on the struct.

**Critique** (unprimed, two rounds). Round one: on the track at 25 m "a grass strip growing between two wheel ruts ... the best result in the set"; at 65 m the strip, then broken by noise, was "olive dashes"; on the gravel road the ruts were "pencil lines" and "pinstripes down the lanes", and no dark region was read as shadow. Changed: ruts cut on the gravel road, the strip made continuous, the track's ruts shallower. Round two: no pinstripes on the gravel road; the track's lanes "hold up well"; but the strip read as "a firm-edged painted stripe" at 65 m and as "a dashed centre line" at 250 m. The strip now fades with the ruts and is checked gone at `track-250`; that was shot and looked at by its author only. What remains:
- at 65 m the strip's olive bed is firmer-edged than the tufts on it;
- the strip takes the bend in a sharp V, as the track does (C65's rounding);
- the track still butts into the road square, and its strip starts on a square cut there;
- the wash beside the road, against wheat, read once as "a soft drop shadow" (medium confidence): it is at the wheat's own luminance and browner.

**Open.** A wide road has no structure across it at the tactical camera: it is a pale ribbon with a textured surface. Pinstripes are not the answer; see SG3.
