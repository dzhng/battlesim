# 20 — Model workbench

**Status:** done. **Depends on:** 11, 12. **Lane:** assets.

**From spike 03:** read [`spikes/03.md`](../spikes/03.md), \"What changes for the consuming slices\", before starting; its findings are part of this contract.

**From slice 11:** articulated bounds are rest pose only, so the raised mast and deployed legs are not covered. Textures are not carried yet, and a GLB with images gets a warning. The workbench must add animated or articulated bounds, and texture support if the art needs it. Load with `AppearanceLibrary.load`; the CLI is `bun run --cwd web asset -- …`.

## Contract

You can drop a GLB in and, within seconds, see validation findings and our own production render, with every view needed to judge a model or animation. The same renderer bakes impostors (decision).

## API seam

- `apps/battle-lab` route `/workbench?bundle=`, registered with its scene.
- The skinned, articulated and static rendering paths are ported from `crowd.ts`, `poseKernel.ts` and `posePalette.ts` into `battle-renderer/src/models/`. These are the production paths, not a second renderer.
- Workbench tools:
  - drop zone;
  - file watch on `assets/source/**`, with re-bake and hot reload;
  - findings;
  - named views (q-front, front, left, rear, top, battle pitch at three zooms);
  - a 1.8 m scale figure;
  - a wireframe of the simulation hit box;
  - socket gizmos;
  - clip and phase scrubber;
  - a feed replay of synthetic observation frames driving the pose driver;
  - stats.
- `asset sheet <bundle>` writes a contact sheet, animation strips and stats. `--accept` copies them to `assets/review/`.
- The impostor far-pose atlas is baked here.

## What you can run or see

`/workbench` plus `bun run --cwd web asset -- sheet <bundle>`.

## Verification

- The scene drops a bad GLB and expects specific findings.
- Sheets are produced headlessly.
- Impostor atlas determinism: same inputs, same hash.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** sheet readability: framing, scale figure, hit box.
- **Reference crop:**

`brokenarrow/steam-brokenarrow-7.jpg`, the infantry crop, for scale at battle pitch.

- **Out of scope:** Model quality itself (slices 21–22).

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Workbench layout and view list.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the user wants a standalone app later, split it out. The loader and renderer stay shared.
