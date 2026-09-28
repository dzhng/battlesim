# C61: rural roads

**Depends on:** C03, C28. **Kind:** slice.

## Question
Do rural roads fade into the ground with a worn shoulder and soft ruts, in two kinds (Q-G2–Q-G4)?

## Contract it unlocks
- C03's `SurfaceArea` road kind gains `dirt_track | country_road` (map data). Speed per kind is in the sim rules, and the village's roads become `country_road` at today's speed, so digests don't move.
- C28's surface bake gains a **distance-to-road** channel. The terrain paints the core (packed dirt, or gravel/asphalt grey), then a **worn shoulder 2–4 m each side** with a noise-jittered edge, then grass.
- Grass density ramps down across the shoulder instead of cutting off.
- **Soft ruts:** two slightly darker, sunken tracks, noise-broken, fading out below ~2 px per rut. A grass centre strip on narrow dirt tracks.
- The shoulder's luminance is at or above the grass's; it differs by hue (L-G3).
- **Round, never blocky (Q-G19):** road centerlines are splines densified to ≤2 m points by the contract's loader (the same curve for sim and look), and shading reads the exact distance, never a grid cell.

## API seam
`contract::map` (road kind), `frame/terrainMaterial.ts`, the C28 bake, `frame/grassPass.ts` density input.

## What the human can run or see
The village road at 25, 65 and 250 m, and a dirt-track lab strip.

## Verification
- Village digests unchanged.
- Terrain GPU ms and grass ms rows.
- Critique gates: "does the road read as an outline?", "does any bend read as blocky?" and "could any dark region read as shadow, or shadow as fog?"

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the road plus 6 m of shoulder either side; exclude plots, grass beyond the shoulder, and units**) against **`../assets/reference/ground/forest-road-summer.jpg` and `manor-path-closeup.jpg`, and the pre-change village road**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Field palette (C71), grass species (C69).


## Delegated to the implementer
Shoulder width within 2–4 m; rut spacing; colours within the kinds. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Road speed; the sim rule is exactly the core.
