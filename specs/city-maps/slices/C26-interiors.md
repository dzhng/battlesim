# C26: interiors

**Depends on:** C25, C15. **Kind:** slice.

## Question
Do rooms behind windows reproduce the repo's mechanism exactly, unlit, within budget?

## Contract it unlocks
Room meshes plus the flat-perspective atlas lookup with stable per-instance room selection by position hash, on every floor of the tiers G0 allowed (O-1). Ground floors show shops. **Emissive zero.**

## API seam
`packages/battle-renderer/src/models/`, the C21 interior metadata.

## What the human can run or see
A facade fixture at 30 and 80 m, and a street-level storefront.

## Verification
- Stable selection under camera movement.
- Frame-cost row within budget.
- Record the O-1 verdict (all room tiers, or LOD0 only).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**window interiors only: "does any window read as lit, or as a hole?"**) against **the Blender render of the same facade (S2 driver, our atlas swapped in)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Facade materials, massing.


## Delegated to the implementer
Nothing beyond the repo's mechanism (Q-E). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Glass unchanged; no walkable interiors.

## Feedback that would change this slice
If the user wants rooms to read darker or lighter, tune the atlas (C15), not the shader.

## Outcome

Yes. Rooms behind windows are drawn by the repo's mechanism (a room box a window, a flat-perspective lookup into a 2 × 5 atlas, the room chosen by a hash of the window's position), unlit, at a cost under the noise. The decisions are in [choices](../choices.md#c26-interiors); what a kit author writes is in the [city readme](../../../packages/scene-assets/blender/city/README.md#interiors).

**The seam.**

- **In the kit:** a room's material names its sheet (`parts.room`), and its box carries the box unfolded as its UVs (`parts.room_box`). This replaces C21's provisional "the UVs are the cell's": projected UVs are wrong between vertices, by up to seven texels on a floor.
- **In the bake:** the catalog names the two sheets' pictures (`interiors`), and every bundle with a room gets its sheet as an ordinary texture, the ten cells laid out four to a row in a 512 px square (`scene-assets` `interior.ts`).
- **In the frame:** a room's triangles are their own index range, drawn with the opaque surfaces by the fragment-less depth and caster pipelines, and by their own stage in the colour pass (`models/surfaceFragments.ts`). It reads the fragment's place in the box off the UV, applies the atlas's pinhole, picks the cell and its mirroring from a hash of the model's position (a flat varying), and shows the picture through the environment's `unlit`: as bright as a matte surface in sun shadow, behind the air, fogged as a face. No sun, no shadow test, no cast light, no emission.

**O-1 verdict: rooms at tiers 0 and 1, on every floor; not at tiers 2 and 3.** The shader would allow every tier. The kit's budgets do not: a far building is one row, and a window there is two to six pixels, which the shell's dark pane already draws.

**Proved on** `/lab/facade`: a block of six flats and three shops.

- **Not a hole, not lit:** from 30 m and 80 m every window's room is brighter than black and at most 0.44 of its sunlit wall's brightness without glass, 0.37 behind it.
- **Unlit:** under three sun azimuths at one elevation a room's picture moves by 1 of 765 while the wall beside it moves by 110 of 255; on a shaded facade the rooms stay under the wall.
- **Stable:** every window shows the same room after the camera has been 250 m away and back, and when the frame is handed the models in the opposite order (difference 0). Six windows show four different cells.
- **The floor strip:** at 80 m and the steep camera a window is a dark, coloured pane with a hint of floor; from 30 m and the street the back wall, furniture and shop shelves read with depth. Acceptable.

**Cost** (the same field, a room behind each of 2,592 windows, at every tier): +0.18 ms at the default camera and +0.14 ms over the whole field, on frames of 2.3 and 2.8 ms: at the edge of the noise.

**Pictures** (`throwaway/evidence/facade/`): `rooms-30m-1920x1080.png`, `rooms-80m-1920x1080.png`, `rooms-storefront-1920x1080.png`, the same three with `-no-glass`, and `facade-street-ahead-1920x1080.png` (the facade in its own shade).

**Compared with** a Cycles render of the same block (`city/facade_lab_render.py`, which does the lookup and the hash on its own in Python): every window shows the same cell with the same mirroring and the same perspective at 30 m, 80 m and from the street.

**The unprimed critique:** nothing glows, and no depth fault, gap or outside of a room box shows.

| Finding | Disposition |
|---|---|
| Rooms on a shaded facade came close to the wall's brightness; the left shop read as nearest to "lights on" | Fixed: a picture is shown as a surface in sun shadow, not in the open |
| Furniture is flat on the back wall and smears along the floor; no parallax between a counter and the shelves behind it | The mechanism (Q-E), as upstream's |
| Two of six windows are near-blank pale rooms; six windows show about three rooms; the shuttered shop is unreadable; chairs stacked on tables read as upside down | The atlas (C15): ten cells a sheet, and what is in them |
| The pictures are much softer than the facade from the street | The atlas's 128 px cells (43 px a metre) |
| At 80 m no room content survives: dark rectangles of differing tint | Accepted: it is what the floor strip gives from above |
| The barred windows alias at 30 m: a pale dashed strip at the sill, bars doubling | A 24 mm bar is one pixel there; the strip is the pale sill between dark bars, and the Cycles render shows it too. The lab's guard, not the rooms |

A second unprimed critic, on the final frames, again found nothing glowing (the nearest case, the shaded facade, fell from medium to medium-low confidence) and the shopfronts reading as rooms from the street. Its strongest finding is the other side of that fix: **at 80 m every window is a flat, near-black rectangle, and the plain upper windows at 30 m are close to it: holes more than rooms.** The first critic said the same of the four darkest at 80 m before the fix. Two levers, neither pulled here: the atlas's floors and depths (C15: from above a window is its cell's floor), and the reference `unlit` shows a picture at (sun shadow now; the open sun made a shaded facade's rooms read as lit).

**Not done:** the `preview-shots` checkpoint; rooms on a real kit, and through the building layer's draws.
