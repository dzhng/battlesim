# 21 — Infantry models and clips

**Status:** done (2026-09-26), with the look gate escalated: the unprimed critique still reads the soldiers as toy-like, and its first ask, printed camouflage, fabric normals and material separation, needs textures in bundles (`choices.md`, slice 21). **Depends on:** 03, 20. **Lane:** assets.

**From spike 03:** read [`spikes/03.md`](../spikes/03.md), \"What changes for the consuming slices\", before starting; its findings are part of this contract. The user chose a real modelling budget, with the critique as a pass/fail gate (`decisions.md`, 2026-09-25).

**From slice 11:**
- The spike's glove shells weight all ten `_leaf` finger joints, which leaves 63 joints against the skeleton's 55. Reweight the gloves, or bake clips from a source that keeps those joints; otherwise `structure.skeleton` fires.
- Add the `stand_aim` clip.
- Rename `rifle_muzzle` to `muzzle` and add an `eye` socket.
- Name meshes with `_LOD<n>`.
- Add a `project-owned` manifest entry for each exported source.

## Contract

Modern rifle, recon and AT soldiers on one skeleton (the rig chosen by spike 03), with Blender-scripted kit:
- helmet, vest and pack;
- a rifle, a DMR for recon, an ATGM launcher for AT.

The core clip roles: idle, walk, run, kneel_fire, prone_pinned, death, plus a static corpse pose. Four LOD tiers and an impostor. Blue and red come from a tint mask, not separate meshes.

**Frozen reference:** **Parity with spike 03 (preserve the winner):** port the frozen `assets/spikes/03/scripts/soldier.py` behaviour first, then fix what the critique named. Meet the open parity row in [`assets/spikes/README.md`](../assets/spikes/README.md) before acceptance.

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

## Verdict (2026-09-26)

**What ships.**
- Rifle, recon (DMR) and AT (launcher) soldiers on one 53-joint rig, as three `skinned` appearances.
- Two clip sets (`quaternius-ubc-rifle`, `quaternius-ubc-launcher`), each with idle, walk, run, kneel_fire, prone_pinned, death and stand_aim.
- Four tiers each: about 26.6–27.5k, 6.9k, 2.35k and 680 triangles.
- Side tint by material mask, and `AppearanceCatalog`.
- The sources are `packages/scene-assets/blender/` scripts over the pinned Quaternius packs.
- The validator is green (fit to authority, sockets, tiers, provenance) and `asset check` passes.
- Parity with spike 03 is met, with named deviations (`assets/spikes/README.md`).

**compare-screenshots.** Telemetry is in `throwaway/slice21/compare/`. The q-front crop of the rifleman was compared with the Broken Arrow infantry crop and with the Defilade soldiers by the wall. The frozen spike rifleman's crop was the "before".
- Against the Defilade crop, grayscale MAE was 80.1 (before 71.7) and edge-energy ratio 1.24 (before 1.44).
- The distance is dominated by the references' grass, snow and scale, so it is not a verdict.
- For this variable (silhouette and kit read, clip readability), the candidate is **less wrong** than the spike:
  - the pack, carrier, helmet and headset silhouette reads at battle pitch;
  - the kinds separate by the launcher and the recon ruck;
  - the spike's white shells, brick boots and aloft rifle are gone.
- Both are still short of the references' surface detail.

**Unprimed critique (last check), six rounds.** Every round said TOY-LIKE; round 6 said "borderline". Acted on:
- body proportions;
- trousers and blousing;
- boots;
- belt, pouches and carrier fit;
- pack straps;
- colour values;
- weapon colour;
- all the clip fixes in `choices.md`.

Not acted on, and why:
- **Printed camouflage, fabric normals and material response.** These need a texture channel; this is the escalated gate.
- **The library's run cadence and the Death01 fall between sampled phases.** Fixing them means authoring locomotion and a fall from scratch.
- **Floor grid lines read as "a stray rod", and the 1.70 m height.** The grid lines are a workbench overlay, and the height is the simulation's.

**Checkpoint.** Preview was opened at 02:31 and closed at 02:34, about three minutes rather than the usual five. There was no reply, so this was decided on the evidence.

**Frame cost.** Row 21: no battle frame change.

