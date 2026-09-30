# C45: street models

**Depends on:** C44, C11, C74. **Kind:** slice.

## Question
Does each prop's model fit its body?

## Contract it unlocks
Appearances:
- theirs, from C11's standalone street kit;
- ours: cars, wrecks, Jersey barrier, bus shelter, scaffold, Heras fence panel, skip bin, pallet stack, site cabin, traffic cone and road barrier, generic with no brands or plates. **Project-owned Blender models only:** the threejsassets.com Construction Site Kit (https://threejsassets.com/packs/construction-site) inspired the construction kinds, but its licence forbids redistribution and our repo is public, so none of its files may enter the repo;
- street trees from the one tree generator (C73/C74 species).

Each has LODs and its terminal state.

## API seam
`packages/scene-assets` appearances.

## What the human can run or see
A contact sheet per kind.

## Verification
- `asset check`; provenance; fit validation.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**body/model fit only; for the construction kinds also silhouette and read at 65 m**) against **the kind's body box overlay; for construction-site kinds, also the matching thumbnail in `specs/city-maps/assets/reference/threejsassets-construction/` (reference only, see its README)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Placement density.


## Delegated to the implementer
Model details within 'generic'. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C44's body values.

## Feedback that would change this slice
Street models that disagree with their body bounds change appearance fit while keeping physical rules fixed.
