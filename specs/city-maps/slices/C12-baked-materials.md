# C12: baked materials

**Depends on:** C11. **Kind:** slice.

## Question
Do the graphs' materials, baked into our three texture slots, keep the facades' character?

## Contract it unlocks
Materials bake into albedo, normal and ORM with box-projected UVs, the grime burned in (Q-H′), at the kit texture edge G0 chose, in the array G0 chose. This never inflates the shared array (L8).

## API seam
`packages/scene-assets/blender/city/`.

## What the human can run or see
An 80 m facade contact sheet per archetype.

## Verification
- `asset check`.
- Texture bytes.
- No false wear from tint alpha (L9).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**facade surface only: brick, stone, grime; exclude windows, sky and ground**) against **`throwaway/evidence/city-maps-chiro/sheet.jpg` (80 m column; regenerate it with `specs/city-maps/assets/render_rts_views.py` if it is missing)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Glass, interiors, massing, far tier.


## Delegated to the implementer
Texture packing within the edge. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Geometry, lighting, glass and interiors stay fixed.

## Feedback that would change this slice
A material that loses wear/tint or scale changes its source bake while retaining the transport channel contract.

## Outcome (China)

Nine ambientCG sets at 1K, pinned in `packs.json`, are baked to 256 px (the edge of every texture shipped today) as nine recipes shared by the kit's 38 materials: 27 images, 1.8 MB. Grime is in the stucco, concrete and roof recipes. No material names a wear colour and vertex-colour alpha is zero. Glass, frosted glass and PVC are opaque; decals and leaf cards are left out ([choices](../choices.md#c11c12c13-the-china-apartment-kit)). The facade keeps its character at 30 m and 80 m in the reassembly; what it loses is the curtains behind the glass.

## Outcome: surfaces that are not opaque (China)

The kit's exporter builds its own materials, so it says coverage and rooms through the same two calls the helpers use (`textures.surface` for a cutout or blended material, `parts.room` for a room), and `asset validate` prints each as intended: `cn_glass` blended at 0.35; `cn_streak`, `cn_leaves` and the `_cut` variant of each bar colour cutouts at 0.5 with a coverage image; `cn_room` and `cn_shop_room` opaque with an interior sheet.

Three recipes are added, ours and procedural (the graph's own decal and leaf images are not copied): `cn_bars` (the shared grille's bars and rails in white over half a metre, so each cage shows its own paint and a bar is twelve texels across), `cn_streak` (a stain, fitted to its decal, drawn in the wall's own colour through the row's tint) and `cn_leaves`. The three share one occlusion-roughness-metalness image, so they add seven images, not nine. A room's UVs are the unit box unfolded round its back wall, written by the exporter as `parts.room_box` writes them.
