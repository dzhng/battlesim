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
