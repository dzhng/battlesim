# 26 — Smoke, fire and dust

**Status:** planned. **Depends on:** 25. **Lane:** battle.

## Contract

Filmic fire and volumetric-looking smoke like Broken Arrow's, and WARNO's dust walls. Burning wrecks smoke. Lifetimes are bounded, and smoke never adds gameplay occlusion.

## API seam

- `EffectLifetime`: bounded particle lifetimes derived from visible events and known wrecks.
- Soft, depth-faded, sun-lit particles.
- Reset, visibility loss and pause behave.

## What you can run or see

`/battle/village` aftermath, paused and playing.

## Verification

- Tests: bounded resources; reset clears everything; smoke doesn't block sight in the simulation (it doesn't exist there).
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** smoke and fire volume and light.
- **Reference crop:**

- `brokenarrow/gameplay-trailer-14.jpg`, the upper-central explosion and smoke trails.
- `warno/steam-warno-1.jpg`, the right-middle fireball and ground dust.

- **Out of scope:** The burst moment (slice 25) and scars.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Lighting model and particle budgets.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If smoke hides units too much for play, lower its opacity in the fixture.
