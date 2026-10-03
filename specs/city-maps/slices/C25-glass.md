# C25: glass

**Depends on:** C24. **Kind:** slice.

## Question
Does glass composite correctly and affordably?

## Contract it unlocks
Alpha-blended model surfaces in the existing frame owner, with an explicit ordering and depth policy. No refraction and no emission.

## API seam
`packages/battle-renderer/src/models/`, the frame's pass order.

## What the human can run or see
An overlapping-windows fixture, and the China enclosed balcony at 30 m.

## Verification
- Opaque occlusion, both sides of glass, fog boundaries.
- Overdraw and GPU ms within G0's budget.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**transparency composition inside window frames only**) against **a Blender render of the China enclosed-balcony module at 30 m (S2 driver, our atlas). The user's own reference shot of that balcony was shared only in chat; if they drop it into `specs/city-maps/assets/reference/`, add it as a second target**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** What's behind the glass (C26).


## Delegated to the implementer
Sort granularity (bounded by S3). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No new transparency framework beyond this.

## Feedback that would change this slice
Glass that disappears or reads opaque at the target camera changes its coverage/reflectance; interiors remain separate.

## Outcome

Yes, with the look left provisional. Glass composites correctly and costs nothing measurable; whether it reads as glass is the open half. The decisions are in [choices](../choices.md).

**The seam.** A blended material's triangles are their own index range (`models/surfaceParts.ts`). They draw last in the frame's existing world pass, after the water, through one pipeline (`models/surfaceFragments.ts`):

- **order:** as the layer packs them, never sorted;
- **depth:** read, not written; in no prepass, so the overlays, the fog's tile cull and the x-ray do not know a pane is there;
- **shadow:** none;
- **light:** the frame's one shade function, with the pane's shading normal turned toward the eye and its own light capped (`presentation.glass`: `turn`, `glint`);
- **over what is behind it:** by the material's coverage value (its opacity), the same from both sides;
- **fog:** it leaves the mask alone. What is seen or unseen is what stands behind the pane.

A kit author marks glass with `coverage=("blended", opacity)` on a material helper and models the pane as one face (`parts.sheet`).

**Proved on** `/lab/facade`: three panes each half across the next, a fourth turned round, and a block's windows.

- **Through it:** one pane keeps 0.77 of the brightness behind it, two keep 0.59, and a pane seen from its back the same as from its front.
- **Opaque occlusion:** a frame in front of another pane's glass is drawn exactly as without the glass.
- **Order:** the same modules handed to the frame in the opposite order draw the same picture to 1 of 255 on every pixel.
- **Fog:** panes in front of unseen ground show it hatched, their frames seen; a pane past the sight line is unseen with the rest.
- **From the street, under four suns** (the fixture's, behind the camera, ahead of it, along the street): a window's pane is 0.25 to 0.42 of its wall's brightness, never a pale plate.

**Cost** (the same field of 288 blocks, a pane in each of its 2,592 bays): +0.16 ms at the default camera and −0.02 ms over the whole field, on frames of 2.3 and 2.7 ms: noise.

**Pictures** (`throwaway/evidence/facade/`): `glass-close-1920x1080.png`, `glass-tactical-1920x1080.png`, `glass-fog-close-1920x1080.png`, `facade-street-fixture-1920x1080.png`, `facade-street-ahead-1920x1080.png`, `rooms-storefront-1920x1080.png`.

**Compared with** a Cycles render of the lab's panes and block from the scene's cameras, not the China balcony the slice names: that kit's glass is still the opaque stand-in. The overlaps darken in the same steps and the frames occlude alike.

**The unprimed critique:**

| Finding | Disposition |
|---|---|
| Panes read as smoked film, and a building's windows as open holes: no reflection, sky tint or highlight says a pane is there | Open. The mirror is bounded on purpose (a grazing pane was a pale plate), which leaves a pane to read by its darkening alone. The knobs are `presentation.glass`; the better cue is the pane's own material (a recipe with a faint waviness in its normal and a dust film in its coverage), which is the kits' pass |
| A pane over unseen ground showed a second, unhatched plate inside it, and seemed to reveal seen ground | Fixed: glass no longer writes the fog mask |
| A pane's tint is close to a cast shadow's colour; glass casts none itself; free-standing panes are dark tiles at the default camera | Left. On a facade a pane sits in a frame against a wall; the lab's free-standing panes are a test of composition |
| The right-hand windows read as plated, rooms do not answer the sun, rooms seem lit on a shaded facade | The rooms' ([C26](C26-interiors.md)) |

A second unprimed critic, on the final frames, saw the hatch run unbroken through the glass and the overlaps combine and sort correctly, and repeated the first finding: smoked plastic, and grey tiles at the default camera.

**Not done:** the `preview-shots` checkpoint; the China balcony comparison; glass on a real kit.

## Outcome: glass on the China kit, and the balcony comparison

The China apartment kit's `M_CN_Glass` is blended glass (`china.py`, opacity 0.35, a cool grey, roughness 0.08), one face a pane, as the graph models them. A frosted pane and a shop door's strip curtain stay opaque; they are there to hide what is behind them. From tier 2 out a window is an opaque pane on a flat wall, in a grey chosen to stand for glass over a room at that distance.

**The enclosed balcony at 30 m** (the line-up's `close` station, `china-apartment-slab-47x11-5f`) against the graph's own render of the same building at 30 m: the same things are there in the same places. The laundry hangs behind the glazing, the balcony door and the room behind it show through two layers of glass, the parapet's inner faces and the floor slab close the box, the neighbouring open balconies have their rails and pot plants. Three differences remain. The graph's glass is pale and reflects the sky; ours adds almost nothing over what is behind it, and an unprimed critic read the windows as openings with rooms behind, glass-less, with a faint film on the balconies and one pane with a sheen. The graph's aluminium frames are white; ours take the graph's per-window tint (green, brown), as the committed kit did. The graph's rooms are its own two furnished boxes; ours are the atlas's cells.

The fixture's "smoked film" does not appear on real art at this opacity. What is missing is the opposite: a cue that there is a pane at all. That is the look's (`presentation.glass`, a sky reflection that reads from the play camera), not the material's: raising the opacity to show the pane brings the film back.

**Not done:** `compare-screenshots` numbers on the balcony crop (the two pictures are from different cameras and lights; the comparison is by eye); the `preview-shots` checkpoint.
