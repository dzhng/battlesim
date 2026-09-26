# 11 — Appearance bundle format, validator, CLI

**Status:** planned. **Depends on:** 03. **Lane:** assets.

**From spike 03:** read [`spikes/03.md`](../spikes/03.md), \"What changes for the consuming slices\", before starting; its findings are part of this contract.

## Contract

Port the appearance-bundle **contract** from `~/dev/game/packages/soldier-assets` (`ART_INPUT_CONTRACT.md`), with our own compact encoding (decision). A bundle is atomic, validated, provenance-checked, and served locally under COEP.

## API seam

- `packages/scene-assets` defines the schema:
  - `SkinnedBundle {skeleton, tiers[4], materials, bounds, far_pose, corpse_pose, sockets}`;
  - `ArticulatedBundle {nodes[{name, parent, pivot, bind, tiers}], bounds}`;
  - `StaticBundle` (with states, for example intact and ruin);
  - `SkeletonClips {id, joints, clips[{name, loop, duration, stride_m?, markers}]}`, binary and shared per skeleton.
- The loader admits complete bundles atomically, and keeps the installed generation on failure.
- An isomorphic `validate.ts` returns `Finding {code, severity, message, fix}` for:
  - structure: one skin, ≤4 weights, no external buffers, 4 tiers finest first, explicit loop flags;
  - basis: Z-up, +X forward, origin at ground, minZ 0 ± 1 cm;
  - fit to authority: soldier height, eye and muzzle sockets against `physics`, vehicle bounds against the hull extents, muzzle against `tank_muzzle_local_m`, within catalog tolerances;
  - required nodes: `turret`, `gun`, `muzzle`, `hmg`, `wheel_*`, `track_L/R`, `deploy_leg_*`, `deploy_mast`;
  - provenance: the hash is in the manifest with an allow-listed licence.
- The loader and CLI detect LFS pointer files and print the exact `git lfs pull --include=…`.
- The CLI `web/asset.mjs`: `validate | bake | check | provenance | pull | blender <script>`. The Blender runner pins 5.2.1.
- Layout: `assets/source/` and `assets/third-party/` (LFS), and `assets/runtime/<hash>/` (LFS, Vite `publicDir`, copied into production builds). The catalog and provenance are text files.

## What you can run or see

`bun run --cwd web asset -- validate <glb>` prints stats (triangles per tier, bones, bytes, bounds) and findings.

## Verification

- Ported tests: gltf import, animated bounds, engine basis, appearance.
- One golden-failure GLB per finding code.
- A deterministic bake: the same hash twice.
- `assetServing.test.ts`: a production build contains every catalog file, served same-origin.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Internal encoding details and the CLI's output format.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If validation rules reject acceptable art, adjust the catalog tolerances, not the rules.
