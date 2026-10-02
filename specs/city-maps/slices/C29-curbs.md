# C29: curbs

**Depends on:** C28. **Kind:** slice.

## Question
Do curbs give sidewalks their edge without cracks?

## Contract it unlocks
Presentation curb geometry along sidewalk boundaries; no navigation step mechanic.

## API seam
`packages/battle-renderer/src/terrain/` or the static chunks.

## What the human can run or see
A grazing street-edge crop.

## Verification
- Joins, intersections, slopes, no cracks.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**curb relief only**) against **C28's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Markings.


## Delegated to the implementer
Curb profile as data. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Movement on authoritative terrain.

## Feedback that would change this slice
Curbs that obscure vehicles or read as walls change curb silhouette within the accepted street cross-section.

## Outcome

**Built as shading, not geometry** ([`choices.md`](../choices.md)). A street's edge is a line of kerbstones and a shaded step up to them. The ground is not moved and no mesh was added, so there is nothing to crack at a join, nothing a vehicle stands behind, and movement is untouched.

**The seam.**
- **`roads.<kind>.curb`** (optional; the summer `road` row has one): the stones' palette, their width (0.3 m) outward from a stroke's edge, a stone's length (1 m) and its joint; the face's width (0.15 m) inside the edge and its tilt (15°; the biome refuses more than 40°).
- **`groundCurb(footprint, paved)`** returns `(slope east, slope north, stones)`: how much of the ground is kerbstone, and the slope the face gives the shading normal, away from the stroke's centreline. It is whole while a pixel is under a quarter of the stones' width and gone at 0.6 (whole at 65 m, gone by 250 m).
- **No curb where another carriageway covers the edge:** across a street's mouth onto another road, and where two streets meet (their strokes are one union).
- **`groundRoadRelief`** is the ruts' slope and the curb's together; the terrain fragment adds it to the shading normal where it added the ruts'.

**Measured** (`STREETS_ONLY`, generated `junction-65`, in the sun):
- The kerbstones read 0.394 against the walk's 0.334 and the roadbed's 0.156.
- Against the pavement alone, the curb moves no pixel by more than 32 grey levels at any station from 25 m to 250 m (mean grey difference 0.05 to 0.25 of 255, before its fade was shortened): it is a line, not a band of its own tone.
- At `overview-2500` the frame with the curb is byte-identical to the frame without.
- Frame cost: in C28's row (the roads' wear on against off, +0.06 to +0.18 ms); the curb has no switch of its own.

**Critique** (unprimed, two rounds). Both found the kerb continuous along straight runs and round corners, with no gap, crack or doubled line, and nothing that reads as a wall or could hide a vehicle. Round one, at a 35° face: "a near-black hairline on one side of the street only", "ink", and a kerb that ended in a spike past the walk at a street's mouth. Changed: 15°, joints between the stones, the mouth (C28's `join_m`). What round two still says:
- **From the ground camera the kerb reads flat**, "a pale edging strip or painted line": no face is visible, and a building's shadow crosses walk, kerb and asphalt in one straight line. It reads slightly raised only on the side turned from the sun, so the two sides of a street differ.
- The dark foot line on that side is a pixel wide at 65 m.
- At 250 m the walk and kerb were "a pale outline drawn round every street", and the kerb's highlight came and went with a street's direction. The curb now fades out before 250 m; that was shot and looked at by its author only.
- Corners are sharp mitres with no radius.
- At a street's mouth the kerb ends in a chamfer a little short of the walk's end.

**Open.** Whether a town needs a kerb that reads as a step from the ground camera. It would be real geometry in the static chunks, or a parallax term; the cut order puts curbs second to go, and nothing here hurts the read from 65 m up.
