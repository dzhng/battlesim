# 16 — Summer terrain material

**Status:** planned. **Depends on:** 13. **Lane:** renderer.

## Contract

WARNO's painterly summer patchwork: fields, plots, verges and roads, as a biome material on our authoritative triangles (landmine 11). There is no bilinear resampling and no relief exaggeration.

## API seam

- Terrain and static props become separate layers, or carry a per-vertex flag, so `FogTerm`'s `isGround` is true only on ground. Slice 12 left the static world mesh mixed, flagged as ground throughout.
- `TerrainSurface` consumes the Rust triangle export plus `fixtures/biomes/summer.json {plots, palettes, road, verge, field_rules}`.
- Ported from `terrain.ts` and `terrainMaterial.ts` as an `adapted` or `technique` port.
- The biome is data, so winter is a new file later.

## What you can run or see

`/battle/village` in a ground tour at strategic and default heights.

## Verification

- Tests: heights and normals at triangle edges and prop contacts match `WorldView`; roads stay where the simulation has them.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** the ground material patchwork at strategic height.
- **Reference crop:**

- `warno/gameplay-tutorial-14.jpg`, the mid-field patchwork (roughly x 380–1500, y 230–620).
- `warno/steam-warno-9.jpg`, the lower-left fields excluding water.

- **Out of scope:** Grass, trees, units, scars and fog.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Plot generation and palette detail, within the biome file.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the patchwork reads too regular, adjust `field_rules` only.
