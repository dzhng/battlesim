# C11: kit modules

**Depends on:** C10. **Kind:** slice.

## Question
Do the kit modules of all three archetypes enter the existing bundle system as validated modules?

## Contract it unlocks
- `packages/scene-assets/blender/city/export_kit.py` (5.2 input form, L6) exports each archetype's unique modules (from S2's inventory) as bundle v3 with `_LOD0..3` tiers. Street and sidewalk outputs are off (L5).
- It remaps China's tint alpha (L9) and splits vertices at material seams (L11).
- Sign text comes from a project-owned invented-name list (Q11).
- Their street kit (`CNK_Street`: lamp, bench, bollard, bins, hydrant, utility box, scooter, planter; NYC `P_00_Lamp`) is exported standalone for C45. **Not their tree.**
- Module-specific validation: a module never claims to be a whole building for footprint checks (`validate.ts:982`).

## API seam
`packages/scene-assets/blender/city/`, `web/asset.mjs` (an `export-kit` subcommand, pinned 5.2.1).

## What the human can run or see
A workbench contact sheet per module (neutral material).

## Verification
- `asset check`.
- Unique-module count matches S2.
- Triangles per module per tier.
- Kit bundle bytes ≤50 MB.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**per-module silhouettes, geometry fidelity only (materials are C12)**) against **S2's Blender contact sheet for the same modules**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Materials and colour (C12); any assembled building (C13, C22).


## Delegated to the implementer
Bundle grouping; module naming. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
`TEXTURE_MAX_PX` stays 1024; the existing catalog; no atlas bytes.

## Feedback that would change this slice
A module silhouette or scale mismatch changes the exported module source before template assembly.

## Outcome (China)

*The numbers below are the first export's. The set now also holds the U and court blocks ([S5](../spikes/S5.md)) and every template's damage state ([C14](C14-damage-placements.md)): 115 modules, 179,575 / 105,566 / 56,220 / 13,545 triangles, a 41.4 MB source and a 38.6 MB bundle; and, last, its rooms, glass and cutouts (the section at the end).*

`packages/scene-assets/blender/city/china.py` exports the China graph as `assets/source/city/china_apartments/`: 74 modules (69 kit meshes and five template shells) with four tiers each, 75,909 / 51,718 / 24,782 / 6,416 triangles, a 19.3 MB source and a 21.4 MB bundle. Street outputs are off, tint alpha never reaches vertex colour, vertices are split per material. `asset validate --unit kit` has no findings.

Not done here: New York and Paris; the street kit for C45; an `export-kit` subcommand (the script runs through `asset blender`). The module count is 74, not S2's 90: S2 counted rooms, curtains, decals, plants and street props, which are not exported ([choices](../choices.md#buildings-lane)). The workbench cannot draw a kit yet, so the pictures are a Blender reassembly from the two exported files (`city/assemble.py`) beside the source graph, with an unprimed critique; `compare-screenshots` telemetry was not run.

## Outcome: rooms, glass and cutouts restored (China)

What the first export left out for want of a path is back, at the two fine tiers:

- **Rooms.** The graph stands a room box behind every window and shop front. Ours is the same box (one module for apartments, one for shops, a row a window, scaled to the bay, the floor and the depth), showing a cell of the interior atlas. Each is fitted to the plan: no deeper than half the building less a clearance, and near a corner no deeper than its distance from it, so two rooms never share space. A room covers its opening top to bottom; an opening with none (the entrance) keeps its dark lining. A ground-floor room's floor stands 12 cm over the ground on a dark threshold. The graph's furnished shop interiors stay out.
- **Behind glass.** The washing and the door inside a glazed-in balcony, and the graph's curtains (at a probability of 0.35, not 0.75, and faded: at 0.75 with the atlas's own curtained cells most windows showed cloth and no room).
- **Cutouts.** A cage's, a flat grille's and a railing's thin bars are one face a plane in the bars' own colour; the frame they hang in stays geometry. The rain stains under sills come back with the graph's `Weathering` at 0.5 (off the openings and off the ground), in the wall's own colour a shade down; the pot plants and roof planters come back with leaf cards.
- **Left as they were:** solar racks (their tubes are 6 cm and tilted, no sheet stands for them), the frosted pane and the strip curtain (opaque), and everything at tiers 2 and 3.

| template | rows | tier 0 | tier 1 | tier 2 | tier 3 | before: tier 0 | tier 1 |
|---|---|---|---|---|---|---|---|
| slab-35x11-4f | 457 | 46,952 | 13,968 | 2,478 | 618 | 46,082 | 11,972 |
| slab-47x11-5f | 735 | 76,526 | 21,716 | 3,724 | 866 | 74,133 | 18,784 |
| slab-59x14-6f | 1,136 | 111,654 | 34,538 | 5,644 | 1,188 | 101,971 | 29,062 |
| slab-53x14-8f | 1,376 | 135,995 | 43,710 | 7,260 | 1,468 | 119,466 | 36,472 |
| point-20x20-7f | 682 | 64,988 | 20,263 | 3,382 | 730 | 58,041 | 16,847 |
| block-u-5f | 1,123 | 110,300 | 34,019 | 5,882 | 1,488 | 100,187 | 28,397 |
| block-court-6f | 1,559 | 147,512 | 48,324 | 7,748 | 1,978 | 140,298 | 41,896 |

The court block is within its budget by 2,488 triangles at tier 0, with its plants, air conditioners and washing a little thinner than a slab's. The damage states are unchanged in kind (no glass, no rooms; a thrown-down cage is its sheets, charred) and stay within their intact states ([C14](C14-damage-placements.md) has their table). The kit is 127 modules, 176,836 / 103,265 / 55,944 / 13,721 triangles, a 41.1 MB source and a 43.5 MB bundle (38.6 MB before): three cutout recipes add 2.4 MB and the two interior sheets 2.8 MB, and the sheets that replace bars take 0.4 MB off the geometry. No ray of 2.58 million shot at the seven templates' facades, with glass and cutouts taken away, gets past a room or onto the back of anything.

One unprimed critique of the street, tactical and close frames. The fixes that followed it were judged by eye only.

| finding | disposition |
|---|---|
| windows read as openings with rooms behind, not flat and not lit; the glass itself reads as almost nothing | kept: at an opacity that shows the pane it is a film. The cue wanted is a reflection, which is the look's ([C25](C25-glass.md)) |
| grass grows inside the shops | the line-up lab's: its ground is a field under every template. In the town the blades are not there; a ground-floor room's floor is lifted 12 cm so the ground itself does not show through it |
| white bars have a saw's edge | fixed: the bars' image covers half a metre, not one, so a bar is twelve texels across |
| stains on the grey tower are tan brackets with a bar across the top | fixed: the wall's own colour a shade down, lanes with no bar |
| curtains flat and over-saturated; many windows a block of colour | fixed: 0.35 of windows, faded |
| frosted windows and strip curtains the brightest things on the wall | fixed: darker |
| forced to tier 1 at the play camera, a cage is cards hanging off its window | kept: tier 1 begins at 130 m, and a frame there costs the court its budget |
| at tier 1 a window is a blue-grey veil, at tier 2 a dark slot | kept, the first as accepted for rooms at distance; the far pane was lightened to meet it |
| the same four pieces of washing on every balcony; a blank mustard quilt over some windows | the graph's kit: one mesh each |
| thin bars and dense grids that may shimmer: dark cages over their own window frames, the roof's tile, shutter slats | not judged: stills only |
