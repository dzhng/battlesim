# 24 — Vehicles, buildings, ruins and wrecks in battle

**Status:** planned. **Depends on:** 05, 13, 22. **Lane:** battle.

**From slice 15:** vehicles are units and are never fogged, so bind the `units` fog group. Buildings, ruins and wrecks are world geometry and keep `faces`.

**From slice 19:** fitting each prop appearance to its placed prop's box is this slice's job; trees are fitted by placement scale. Consider converging slice 19's scenery layer (scale, tint, tier selection) with the models layer's instanced props, so there is one instanced-scenery owner.

## Contract

Tanks and trucks are articulated from the feed:
- turret and gun from each mount's bearing and elevation;
- recoil on shot deltas;
- wheels from velocity;
- track scroll;
- deploy nodes from deployment progress.

Houses, ruins and wrecks render from `known_props` only: an unseen collapse stays unseen.

## API seam

- `VehicleRig {turret, gun, hmg, wheels, tracks, deploy}` consumes `WeaponPose` and observed motion.
- `PropAppearance` maps props and `KnownProp.replaces` to bundle states atomically.
- The remaining `proxies.ts` and `unitProxies.ts` are deleted.

## What you can run or see

`/battle/village`, a drive and fire sequence, and a collapse replay.

## Verification

- Tests:
  - node axes and elevation;
  - recoil counters;
  - wheel motion;
  - an unseen destruction stays unseen;
  - known replacement is atomic.
- Picking unchanged.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** mechanical articulation and structure read.
- **Reference crop:**

- `warno/steam-warno-1.jpg`, the nearest tank.
- `defilade/x-shader-craters-f4.jpg`, the rubble left of the house (ruin form).

- **Out of scope:** Effects and grass.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Recoil and suspension feel.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If articulation reads wrong, fix the mapping only.
