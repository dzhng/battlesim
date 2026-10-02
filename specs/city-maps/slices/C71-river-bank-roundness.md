# C71: river bank roundness

**Depends on:** C70. **Kind:** slice.

## Question
Do banks read round, with the 4 m flat-shaded facets hidden (Q-G19)?

## Contract it unlocks
On bank pixels the shading normal comes from the river's analytic bank profile, not the facet (SG2's fallback 2, if GG took it). Normals only; the sim's triangles are untouched. The tilt never exceeds 40° (the avoid-list).

## API seam
`terrainMaterial.ts` bank shading.

## What the human can run or see
`bend-25`, and a low-pitch shot along the meander.

## Verification
- The maximum facet luma step is under GG's bound.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the bank band only; critique: "does any river or bank read as blocky or stepped?"**) against **C70's shot (facets visible)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Colour bands, water.


## Delegated to the implementer
Blend width. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C70's colour masks.

## Feedback that would change this slice
A visibly angular shoreline changes bank geometry sampling; water colour and shading remain separate owners.

## Outcome

**Banks read round; the shading normal was C69's, and this slice bounds it, keeps its check and adds its evidence.**

**The seam.** `groundBank` (terrain material) gives the bank's shading normal from the river's cross-section, as it has since C69. One line changed: the slope it shades by is the bank's true slope times the biome's `shore.relief` (0.5, C70's choice), never past tan 40° (`MAX_SLOPE`, the bound the scars already had). Validation holds `relief` to at most 1. Heights, triangles and the water's edge are untouched. The rig has the low shot along the meander (`river` `meander-low-90`).

**Measured** (river lab, 1920×1080).

| Question | Bar | Measured |
|---|---|---|
| Largest step in the ground's shading, walking each bank a metre at a time at 1, 2.5, 4 and 6 m from the water | 8% of the flat ground's luminance | 1.5% top-down at 250 m (4,393 steps), 1.4% at the play camera (1,252 steps) |
| The same walk with the bank's shading switched off (a scratch copy of this tree) | — | 18.7% and 17.4%; 176 and 52 steps over the bar: the check fails as it should |
| Tilt | 40° | the lab's and the generated map's banks slope 14°, shaded as 7°; the contract refuses a bank past 35° |

**Frame cost: not resolved.** One paired run (this tree against the same tree with the bank's shading off, interleaved in 1.5 s batches, five rounds, two framings) read −0.47 ms at the play camera and +0.74 ms top-down for the shading (medians of the paired differences): two signs, on a machine at a load average of 50 to 120, with 17 frames a batch against the timer's 240-frame window. The shading is one loop over the river stretches the surface field lists for the pixel, which the ground's colour already walks; that is an argument, not a number. The row is owed.

**What fresh eyes found.** Two unprimed critiques of the lab and a generated map (250 m, 65 m, 25 m, low pitch, low sun), asked whether any river, waterline or bank reads blocky, faceted, stepped, scalloped or sawtoothed. On this slice's question: the waterline was smooth in every frame both times, and neither named a facet of the height grid on a bank. What read stepped or scalloped was the bare earth's outer line and, the first time, straight wedges where plots met the bank; both are C70's band and are in its Outcome. With the bank's shading off the same frames show the grid as a sawtooth along every bank and a V through the water.

**Open.** Beside relief the bank is lit as if the land past it were flat (C69's note). The comparison frames with the facets visible came from a scratch copy, not a switch in the lab.
