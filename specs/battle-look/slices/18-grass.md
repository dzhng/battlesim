# 18 — Grass

**Status:** planned. **Depends on:** 16 (17 for trampling). **Lane:** renderer.

## Contract

WARNO-dense 3D grass that sways, is seated on the triangles, and is masked by roads, buildings and forests. It bends where the ground layer says it's trampled.

## API seam

- **Workbench (user, 2026-09-26):** every grass kind (the blade or card set per plot kind) is a `scene-assets` appearance that `/workbench` can load and show: a sheet at the battle views, a turntable, and wind motion if any. The field generator stays in code; the instanced unit is an appearance.
- Port `grass.ts` and `grassField.ts` (near, mid and far tiers).
- Per the Ghost of Tsushima technique ([research](../research.md)): GPU blades, one wind field, staged culling.
- Parameters in `biomes/summer.json.grass`.
- Grass never changes sensing.

## What you can run or see

`/battle/village` in a near-to-far traverse.

## Verification

**Kill gate first:** the grass pass costs no more than 4 ms at ground zoom, and there is no tier popping visible to the critique.

**Fallback:** near cards or shell grass within about 60 m, plus the painterly ground beyond.

Tests: seating on the triangles; stable residency; trampled response.

Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** grass density, height and sway.
- **Reference crop:**

`warno/steam-warno-1.jpg`, the foreground grass left of the nearest tank (lower band).

- **Out of scope:** Vehicles, trees and fog.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Density, sway and LOD distances, within the biome file.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the grass reads as noise at strategic height, lower the far tier's density.
