# 14 — Units, poses, corpses and aircraft

**Contract:** native draws every soldier and vehicle the browser draws, posed as the browser poses them. That covers own squads member by member, identified enemies, corpses and falling aircraft. Wrecks are known props, drawn by slice 15's props path, as in the browser.

**Seam:** a port of the browser's per-frame derivations, on the frame record's `simTime`:
- `web/src/battle/present/interpolate.ts`: per-soldier blending by id between published ticks;
- `poseFeed.ts`, `poseDriver.ts`, `poseKernel.ts`, `clipTable.ts`, `articulation.ts` and `drawnModels.ts`:
  - hull pitch and roll from acceleration;
  - track and wheel roll from distance and turn;
  - turret bearing and eased elevation;
  - gait and facing from velocity;
  - the corpse limit.

**Data the browser exports in the manifest:**
- `presentation` pose tuning from `fixtures/game.json`;
- each appearance's articulation and clip data from `scene-assets`.

Draw through `world/model_batches.gd` (slice 09). Units move per frame, so their transforms are written each frame; articulated parts are batched per part. Port the impostor cards for distant posed soldiers (`models/impostor.ts`), deferred from 09. Remove the own-only unit path and the capsule and side-disc fallbacks from `legacy_map_layer.gd`.

**Runnable artifact:** plate pairs, especially close infantry and vehicle cuts, and paris-corner 1751 (the T-posed soldier).

**Focused gates:**
- **Counts.** The browser exports per-plate counts of drawn models by kind: soldiers, vehicles, corpses and aircraft, read from its model feed in the plate's frame, like the grass counts. Native's counts must equal them on every plate.
- **Poses.** A test checks articulation values at fixed inputs (turret bearing, hull pitch, gait phase) against vectors exported from the TS.
- **Visual.** `compare-screenshots`, then an unprimed `screenshot-critique`, then `preview-shots` (non-blocking).
- **Cost.** Report this layer's pass cost. Re-run slice 12's measurement on market-town and keep native at or below the browser.

**Visual variable:** units: presence, pose and placement.

**Delegated:** how clips are sampled on the GPU or CPU, provided poses match the vectors.
