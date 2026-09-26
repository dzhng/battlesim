# 13 — Light: sky, sun shadows, grade

**Status:** done. **Depends on:** 12. **Lane:** renderer.

## Contract

A WARNO warm afternoon: a physical sky and environment light, cascaded sun shadows retuned for 0.3–1.6 km maps, and an AgX grade with bloom. Box units still stand in.

## API seam

- One `EnvironmentFrame` supplies sky, PMREM, sun direction and cascades to every material.
- `PostSettings` owns exposure, AgX and bloom.
- Values live in `presentation.light {sun_azimuth, sun_elevation, exposure, grade, bloom, cascades}`.
- Sun shadows keep a distinct, directional, object-attached look, so slice 15's fog stays tellable apart.
- As shipped, the block also holds `sun_intensity`, `sky {turbidity, radiance, fill}`, `haze` and `backdrop`; the type and validator are `light/sceneLight.ts`, the owner `frame/environmentFrame.ts` (see `choices.md`, slice 13).

## What you can run or see

`/battle/village` at a fixed afternoon.

## Verification

- Cascades split over the map's actual depth range from the camera, not from its near plane (spike 01, landmine 2): four 2048² cascades capped at 2,600 m, with a per-cascade normal bias, all in `presentation.light.cascades`.
- Look problems the spike's critique found, which this slice owns: shadows reading as a saturated teal decal, a khaki band below the horizon past the map edge, a hard horizon at eye level, and flat exposure.

- Tests: cascade coverage across the village distances, including the strategic camera at 1.6–2.6 km; one sun direction everywhere; finite HDR values; linear/sRGB boundary; disposal.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** light and grade (sky, sun warmth, shadow softness, haze).
- **Reference crop:**

- `warno/steam-warno-1.jpg`, the upper sky band (roughly y 0–560).
- `warno/gameplay-tutorial-14.jpg`, the lower half, for aerial haze and shadow softness.

- **Out of scope:** The flat ground material, box units, grass, trees and fog.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Sun angle, exposure and cascade splits, within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the grade reads too warm or too cold at play, re-tune `presentation.light`.
