# 22 — Vehicle and building models

**Status:** planned. **Depends on:** 03, 20. **Lane:** assets.

## Contract

A modern tank with a cannon and HMG, a supply truck with deploy legs and mast, and village houses sized to the prop half-extents, each with a ruin state. Burnt wreck variants. Muster-style Blender-scripted parts: bevels and edge wear.

## API seam

- `blender/tank.py`: `turret`, `gun`, `muzzle`, `hmg`, road wheels with radius, `track_L/R`, and a UV-scroll track material.
- `blender/supply_truck.py`: `wheel_*`, `deploy_leg_*`, `deploy_mast`.
- `blender/house.py`: intact, plus a ruin at `ruin_height_m`.
- The wreck material variant.
- A test that turret yaw moves the muzzle along its arc.

## What you can run or see

Workbench sheets for the tank, truck, house and ruin.

## Verification

- The validator is green: fit to hull boxes, the muzzle position, required nodes.
- The node-transform unit test.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** vehicle and building silhouette.
- **Reference crop:**

- `warno/steam-warno-1.jpg`, the nearest tank.
- `warno/steam-warno-13.jpg`, the vehicle crop, for the truck's proportion.
- `defilade/trailer-t36s.jpg`, the village houses: form, not snow.

- **Out of scope:** In-world lighting, articulation in battle and fog.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Model detail and style within the Muster technique.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the tank reads as a toy, use a hand-modelling pass. Its style change goes to the user.
