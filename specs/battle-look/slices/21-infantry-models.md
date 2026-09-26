# 21 — Infantry models and clips

**Status:** planned. **Depends on:** 03, 20. **Lane:** assets.

**From spike 03:** read [`spikes/03.md`](../spikes/03.md), \"What changes for the consuming slices\", before starting; its findings are part of this contract. Blocked on the user's modelling-budget call (orchestrator, after spike 03 in `choices.md`); the provisional call is option (a).

## Contract

Modern rifle, recon and AT soldiers on one skeleton (the rig chosen by spike 03), with Blender-scripted kit:
- helmet, vest and pack;
- a rifle, a DMR for recon, an ATGM launcher for AT.

The core clip roles: idle, walk, run, kneel_fire, prone_pinned, death, plus a static corpse pose. Four LOD tiers and an impostor. Blue and red come from a tint mask, not separate meshes.

## API seam

- `packages/scene-assets/blender/infantry_kit.py` and `clips_*.py`. Weapons are skinned to the hand bone at bake time, so there is no runtime attachment system.
- The ported `blender-mesh-lods.py` produces 4 tiers.
- `AppearanceCatalog` maps each kind and side to a bundle.
- Every third-party input is in the manifest with an accepted licence.

## What you can run or see

Workbench roster sheets and strips for all three kinds.

## Verification

- The validator is green: fit to authority, sockets, tiers.
- The bake is hash-stable against the pinned Blender.
- Clip coverage matches the roles table.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** soldier silhouette and kit read, and clip readability.
- **Reference crop:**

- `defilade/x-saving-wounded-f4.jpg`, the two soldiers beside the wall: form, not colour.
- `brokenarrow/steam-brokenarrow-7.jpg`, the infantry crop.

- **Out of scope:** In-world lighting, grass and fog.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Kit detail and polygon count per tier, within catalog limits.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If a kind is hard to tell apart at battle pitch, change its silhouette cue (pack, launcher) only.
