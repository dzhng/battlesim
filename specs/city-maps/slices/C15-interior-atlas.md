# C15: interior atlas

**Depends on:** G0. **Kind:** slice.

## Question
Is there a project-owned, dim, daylight-only interior atlas in the repo's 2×5 layout?

## Contract it unlocks
A numpy or Blender recipe renders rooms and shopfronts (ground floor) with no lamps. The atlas is ours, generated in the repo (Q-E).

## API seam
`packages/scene-assets` texture recipe.

## What the human can run or see
The atlas sheet.

## Verification
- `asset check`.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the atlas as a whole: "does any room read as lamp-lit or glowing?"**) against **the repo's atlases for **layout only**; never copy content**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** How it looks behind glass (C26).


## Delegated to the implementer
The room and shop content within 'dim, daylight, generic'. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No photos; no lamps.

## Feedback that would change this slice
An interior atlas that reads as repetition or a real brand changes its project-owned source, not facade geometry.

## Outcome

Yes. `packages/scene-assets/blender/city/interiors.py` builds ten apartment rooms and ten ground-floor shops as scripted geometry, lights each with nothing but the sky through its own window wall, and renders it from the lookup's pinhole. It writes `assets/source/city/interiors/rooms.png` and `shops.png` (Git LFS): 256 × 640 px, 2 × 5 cells of 128 px. The contract a shader reads them by is in the [city readme](../../../packages/scene-assets/blender/city/README.md#interiors); the decisions are in [choices](../choices.md#c15-interior-atlas).

- **Dim:** mean luminance 0.033 linear (0.18 sRGB-encoded) on both sheets, against about 0.7 for a sunlit plaster wall. The brightest pixel is 0.45 sRGB (rooms) and 0.40 (shops); under 2 % of pixels are near black. No material emits.
- **Deterministic:** two runs wrote the same bytes. That was checked on the pass before the last round of room changes, which touched only the rooms and two tone constants; the committed sheets were not rendered twice.
- **Judged by eye and by an unprimed critique**, on the sheets and on a mock-up that runs the lookup behind window-sized openings at 30 m and 80 m. Straight on, windows read as dark, tinted, lived-in rooms, not holes and not lit. From the tactical camera's pitch they are the cell's floor, a dark coloured pane with some streaks (see choices). The critique's findings that were acted on: a veil from lifted shadows, a pale pelmet that read as a strip light, barcode curtain folds, five layouts each used twice, blocky wall patches.

**Retoned after C26** (the numbers above are the first sheets'). In the game a cell is shown as a matte surface in sun shadow, and the dim sheets made every window a near-black rectangle at 80 m. The sheets are now set against that: exposure 1.0 under a ceiling of 0.15 linear, lighter floors, stronger wall hues. Cell means are 0.28 to 0.39 sRGB (were 0.13 to 0.25), the brightest pixel 0.42. On the facade scene a room behind glass stands at 0.28 to 0.45 of the sunlit wall at 30 m and 0.39 to 0.44 at 80 m (were 0.23 to 0.37 and 0.25 to 0.33); the scene's bound of 0.7 was not moved. The committed sheets were rendered twice, by two code paths, to the same bytes. An unprimed critique of the frames is recorded with the decision in [choices](../choices.md#the-sheets-are-toned-for-the-shade-the-game-shows-them-in). Frames: `throwaway/evidence/facade/rooms-*.png`.

**Not done:** the layout comparison against upstream's atlases (they were never opened), the user checkpoint, and `asset check` (the sheets are not in the asset catalog, so it cannot move). **Open for C26:** curtains hung at the window are still the brightest thing in their cells; the shuttered shop is a plain dark slab; whether the floor-strip read from above is acceptable.
