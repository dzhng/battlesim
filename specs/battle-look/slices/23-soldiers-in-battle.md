# 23 — Soldiers in battle

**Status:** planned. **Depends on:** 05, 13, 21. **Lane:** battle.

## Contract

Soldiers in the village are the real animated models, driven only by the observation:
- walk or run from velocity;
- kneel_fire on a shot-counter increment;
- prone when suppression reaches `suppression.collapse_level`;
- death at the corpse, then a static corpse instance, never skinned.

LODs and impostors come from the port; picking keeps the simulation's boxes.

## API seam

- `battle-renderer` `PoseDriver(ObservationView, PresentationClock) -> PoseFrame` owns clip choice and transitions, replacing `~/dev/game`'s timeline (landmine 9).
- Facing follows the decision: velocity, else mount bearing, else unit yaw. Every term is per soldier: each soldier's velocity comes from its own published positions by member id. Nothing reads a formation slot or the squad's heading, because a future spec moves soldiers individually to cover (README firewalls).
- The crowd instance data is fed by member ids.
- The infantry path of `proxies.ts` and `unitProxies.ts` is deleted.

## What you can run or see

`/battle/village` and `/replay/village`.

## Verification

- `PoseDriver` unit tests on packed frames:
  - pause and resume;
  - reacquiring an enemy;
  - reinforcement;
  - pinned then recovering;
  - hidden deaths not animated.
- Endurance with 20,000 static corpses records frame cost.
- `picking.test.ts` unchanged.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** soldier motion and read in context.
- **Reference crop:**

`defilade/steam-4.jpg`, the left-centre infantry near the tank.

- **Out of scope:** Effects, and vehicle models.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Blend timing and LOD distances.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If a pose reads wrong in battle, fix the clip mapping in `PoseDriver`. Rule inputs stay as they are.
