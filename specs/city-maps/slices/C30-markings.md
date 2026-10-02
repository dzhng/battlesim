# C30: markings

**Depends on:** C28. **Kind:** slice.

## Question
Are crossings and lane lines legible at battle distance and never mistaken for tactical marks?

## Contract it unlocks
Deterministic, generic marking placements on roadbed.

## API seam
`packages/battle-renderer/src/terrain/`.

## What the human can run or see
An intersection crop at 65 m.

## Verification
- No sidewalk overlap; no interference with order or tactical ground marks.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**marking legibility only**) against **C28's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Everything else.


## Delegated to the implementer
Generic layout. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Pavement and curbs fixed.

## Feedback that would change this slice
Markings that dominate tactical reading change their width/contrast; street geometry is fixed.

## Outcome

**Built.** Every town street has a dashed centre line, and a crossing's bars wherever another road crosses it. Nothing is placed by a list: the lines are a function of the strokes, so they are the same on every machine and need no data in the map.

**The seam.**
- **`roads.<kind>.markings`** (optional; the summer `road` row has one): the paint's palette, its `cover` and `wear`; the centre line's width (0.2 m) and `dash_m` (1.5 m on, 3 m off); a crossing's bars (0.5 m wide and apart, 2 m long, starting 0.5 m from the crossing road's edge, keeping 0.45 m from their own road's edge).
- **`groundMarks(xy, footprint, paved)`** returns how much paint lies at a point. It needs the stroke's lane and run (`groundPaved`), and reads the cell's strokes once more for the road that crosses: a carriageway's stroke whose direction differs from this one's.
  - **Centre line:** by the distance along the stroke, so it runs unbroken round a bend. A dash that would come within 0.3 m of a crossing's bars is left out whole.
  - **Crossing:** on a road before any stroke that crosses it and runs on past its far edge. A side street has one at its mouth; the street it joins has none there.
  - **Never off the roadbed:** the lines exist only in a stroke's own lane, and the bars keep inside its edge.
- **A line fades by its own width:** whole while a pixel is under 0.35 of it, gone at 0.85. The centre line is whole at 65 m and gone at 250 m; the bars last to about 250 m.
- **`groundReach`'s paved reach** is now 4.3 m (it was the shoulder's 3.5 m): a dash's far end must see the road its near end stops for. The field's index grew by under 1% ([`choices.md`](../choices.md)).

**Measured** (`STREETS_ONLY`, generated `junction-65`, in the sun):
- Paint reads 0.36 to 0.48 against the roadbed's 0.156 (2.3 times and more).
- Along the streets' middle 12% of pixels are paint (a dash is a third of the line, and the line 0.2 m of the 0.4 m band read); in the lanes 5%, the crossings' bars.
- In the roadbed 0.25 to 0.35 m inside a street's edge (between the curb's face and where the bars end), 0 of 892 pixels are paint: no line reaches the curb or the walk.
- Against the pavement alone the lines move 0.1% to 0.3% of pixels by more than 32 grey levels from 25 m to 65 m, and none at 250 m (before the bars were given their own fade).
- Frame cost: in C28's row; the lines have no switch of their own.

**Critique** (unprimed, two rounds; each was asked whether anything painted could be taken for the game's own marks). Both: no paint on a walk, a kerb or the gravel; crossings "read unambiguously as crossings"; against saturated yellow order paint no confusion; against a white dashed mark running along a street, "low to medium" for the centre dashes at 25 m to 65 m. What keeps them apart, in the critique's words: "dull cream, matte, textured, darkened by shadow and locked to the street centre". Round one: a metre-long stub of a dash beside a crossing; sparse dashes; hairline dashes at 250 m; paint, kerb and walk in one cream. Changed: whole dashes, 1.5 m dashes, the fade, whiter paint and greyer stone. Round two: paint that faded across a single 65 m frame, and a mouth with no crossing where the country road bends. Changed after it (shot and looked at by the author only): the bars fade by their own width, and a crossing lies before a bending road too. What remains:
- Every arm of every junction has the same six bars and nothing else: no stop line, an empty junction box. At 65 m the crossings are the strongest thing on the ground.
- A side street has a crossing where it meets the gravel road, an odd place for one (the country road through a town again: C28's Open).
- The space between a crossing and the first dash differs from arm to arm, as the dashes fall.
- The bars' wear reads as faint smudges at 65 m.

**Open.** A white order mark (the zone outline, the supply reach) drawn along a street has not been shot over the centre line; the paint has no glow and a width in metres, which is the argument, not a test.
