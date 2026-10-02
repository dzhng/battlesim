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

*The numbers below are the first export's. The set now also holds the U and court blocks ([S5](../spikes/S5.md)) and every template's damage state ([C14](C14-damage-placements.md)): 117 modules, 180,862 / 104,846 / 55,373 / 13,625 triangles, a 41.4 MB source and a 38.6 MB bundle.*

`packages/scene-assets/blender/city/china.py` exports the China graph as `assets/source/city/china_apartments/`: 74 modules (69 kit meshes and five template shells) with four tiers each, 75,909 / 51,718 / 24,782 / 6,416 triangles, a 19.3 MB source and a 21.4 MB bundle. Street outputs are off, tint alpha never reaches vertex colour, vertices are split per material. `asset validate --unit kit` has no findings.

Not done here: New York and Paris; the street kit for C45; an `export-kit` subcommand (the script runs through `asset blender`). The module count is 74, not S2's 90: S2 counted rooms, curtains, decals, plants and street props, which are not exported ([choices](../choices.md#c11c12c13-the-china-apartment-kit)). The workbench cannot draw a kit yet, so the pictures are a Blender reassembly from the two exported files (`city/assemble.py`) beside the source graph, with an unprimed critique; `compare-screenshots` telemetry was not run.
