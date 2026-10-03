# C24: cutout

**Depends on:** C21, C22. **Kind:** slice.

## Question
Does alpha-cutout keep its coverage through colour, depth and shadow?

## Contract it unlocks
Cutout coverage through the model colour, depth-prepass and shadow paths (`MATERIAL_ROWS` grows, `modelLayer.ts:129-132`).

## API seam
`packages/battle-renderer/src/models/modelLayer.ts`.

## What the human can run or see
A grille and sign fixture.

## Verification
- Matching colour and shadow silhouettes; mip behaviour; MSAA edges; fog-mask behaviour.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**cutout edges only**) against **the same fixture in Blender (S2 driver)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Glass, interiors.


## Delegated to the implementer
Cutoff values as material data. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Opaque wear unchanged; no leaf-card street trees.

## Feedback that would change this slice
Cutouts that alias or vanish change coverage thresholds/mips; glass and interiors stay separate.

## Outcome

Yes. A cutout keeps its coverage through colour, depth and shadow, and the opaque path is as it was. The decisions are in [choices](../choices.md); what a kit author writes is in the [city readme](../../../packages/scene-assets/blender/city/README.md) ("Surfaces that are not opaque").

**The seam.** A material's kind of surface (cutout, opaque, room, blended) is a range of its mesh's indices, ordered once at install (`models/surfaceParts.ts`), and every draw of the models layer and the building layer names the kind it draws. A cutout has three pipelines in the frame's existing passes, with the models' own vertex stage:

- **depth prepass:** a fragment stage that answers the samples it covers (a sample mask), from the coverage value read through the mip chain and scaled so that the material's cutoff is half;
- **sun cascades:** the same coverage, dithered a texel;
- **colour:** the opaque fragment stage at depth equal, so it shades exactly the samples the prepass kept.

A material row grew to five vec4s: the fifth holds the cutoff and the layer of the coverage image. No pass was added and `battleFrame.ts`'s order is unchanged.

**Proved on** `/lab/facade` (scene `facade`): a grille (a quarter there) and a perforated sheet (seven tenths there) as fence panels.

- **Colour and depth:** through a hole the frame is the frame without the panel, to the bit, with and without fog; the metal beside it is the panel's.
- **Shadow:** the grille dims the ground 0.29 as much as a wall does at the same spot and the sheet 0.71.
- **Mips:** at 250 m, where a panel is 15 px, both are still drawn, as veils.
- **Fog:** a hole across a sight boundary shows the hatched unseen ground, and the panels themselves are seen.
- **Opaque unchanged:** `city-block` (kit buildings) and `village-watch` (soldiers, vehicles, village houses) frames on main and on this branch, with all three slices in, differ by what two runs of one tree differ by: at most 689 pixels by 14 of 255 on `city-block`, with its overview, wide and ground-class frames identical (the choices' first entry).

**Cost** (Apple metal-3, 1920 × 1080, paired and interleaved, a field of 288 blocks with a guard over every window and the two panels; the machine was under other sessions' load, so differences under about 0.3 ms are noise):

| View | Cutouts drawn against not | Frame |
|---|---:|---:|
| Default camera over the field (1,280 models) | +0.25 ms | 2.3 ms |
| The whole field from 420 m (4,626 models) | +0.06 ms | 2.8 ms |

**Pictures** (`throwaway/evidence/facade/`): `cutouts-close-1920x1080.png`, `cutouts-tactical-1920x1080.png`, `cutouts-far-1920x1080.png`, `cutouts-shadow-grille-1920x1080.png`, `cutouts-hole-grille_panel-crop.png`, `cutouts-fog-close-1920x1080.png`.

**Compared with** a Cycles render of the same kit from the scene's own cameras (`city/facade_lab_render.py`): bars, rails and holes land on the same pixels at 25, 65 and 250 m; the game's edges carry 0.76 to 0.94 of the reference's edge energy (four samples a pixel against 48), and its shadows are a soft wash where Cycles draws each bar.

**The unprimed critique** found no confusion between a cast shadow and the unseen ground. Its other findings and where each went are in the choices. Two were fixed here: the sample count is rounded only where a pixel resolves an edge and dithered where it does not, and the sheet's recipe is a flat dark plate. A second unprimed critic, on the final frames, found what shows through the openings clean and with no halo, and repeated the standing ones: at the default camera the grille's bars are under a pixel and the sheet is a dark slab, the shadows carry no bars and have grain, and at 250 m both panels are mottled blocks of a few dozen pixels.

**Not done:** the `preview-shots` checkpoint; any look at a cutout in motion. No real kit has a cutout yet, so the building layer's cutout draws have not been seen on a town.
