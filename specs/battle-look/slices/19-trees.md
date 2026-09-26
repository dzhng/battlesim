# 19 — Trees and scenery

**Status:** done. **Depends on:** 16. **Lane:** renderer.

## Contract

Summer tree lines and forests like WARNO's, placed from the authored forests and props. Visual detail creates no colliders and no sensor effects; the simulation's forest volumes stay the authority.

## API seam

- **Workbench (user, 2026-09-26):** every tree and hedgerow kind is a `scene-assets` appearance loaded through the one loader and shown in `/workbench`: sheets, turntables, LOD tiers and impostor, beside the simulation's canopy footprint and height. Placement stays in code; the instanced unit is an appearance.
- Port `scenery.ts` as `SceneryPlacement` from `map.forests` and props.
- Species and detail come from `biomes/summer.json.trees`.
- Trees are drawn with shadows and the fog term.

## What you can run or see

`/battle/village` tree-line tour.

## Verification

- Tests: placement stays inside the forest volumes; resource cleanup.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** vegetation silhouette and density.
- **Reference crop:**

`warno/steam-warno-9.jpg`, the central road with its bordering trees.

- **Out of scope:** Grass density, units, buildings' look and fog.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Species and detail, within the biome file.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

None expected.
