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

## Findings (2026-10-11)

Built in `5f56a47ae`. Control `throwaway/native-parity/2026-10-11T13-23-23-270Z` (slices 14 and 15's browser exports, slice 16's terrain), native plates beside it in `native/`, pairs in `throwaway/slice14/pairs/`.

**Contract.** The manifest gains `units` and `modelCounts` (`web/src/battle/benchmark/unitManifest.ts`).
- `units` holds:
  - the pose feel (`presentation.pose`), the lean hold, the model detail tiers and the side tints;
  - each fielded unit type's facts: half track, airborne, mount roles, per-mount weapon, carrier and active hold;
  - per side, the installed appearance sets `AppearanceCatalog.resolve` picks from, already in the side's faction look;
  - per appearance, a body's joints, clips and far and corpse poses, or a vehicle's nodes and rig (`articulationRig`, computed by the browser);
  - the dense clip tables (`buildClipTable`) as `units-clips-<n>.u32` artifacts. Bodies with the same table share it: market-town has 27 appearances on 2 tables of 1.44 MB.
- `modelCounts` gives each plate's posed models by kind (soldiers, vehicles, aircraft, corpses, fading), from the browser's own pose frame (`modelCounts` in `modelInstances.ts`).

**Counts.** Native's equal the browser's on every plate, 6 of 6 on paris-corner and 18 of 18 on market-town. Market-town runs from 65 soldiers and 3 corpses at its first plate to 8 soldiers and 69 corpses near its end, so deaths, the dying clip's timing and the corpse list are exercised. A difference stops the run at a `models` blocker.

**Poses.** `pose_vectors_test.gd` drives native's port (`units/`) over every frame record of the paris-corner fixture to its first plate. It checks the frames against the browser's own `TickInterpolator`, `ObservationFeed` and `PoseDriver` on the same records (`nativePoseVectors.test.ts`):
- clip, phase, facing, position and crossfade of every soldier, and every vehicle's articulation, on 30 sampled frames;
- the Abrams's `articulate` locals at four fixed inputs (turret bearing, gun pitch and recoil, HMG, track travel);
- a rifleman's joint worlds at five clips and phases.

All agree to 1e-4. Breaking the run threshold, the turret's sign, or the frame weight each fails it. `pose_kernel_display_test.gd` checks the GPU kernel against the CPU port to 1e-4: worst 1.4e-6 on a clip, and on a crossfade too.

**Visual.** Plate pairs are in `throwaway/slice14/pairs/` and crops in `crops/`.
- **Soldiers.** On every plate where both clients show them, soldiers stand, run, kneel, lie, lean and fall pose for pose with the browser's. Paris-corner's T-posed soldier is gone.
- **Vehicles.** Turrets, guns, wheels and antennas agree, and track textures scroll.
- **The one bug the plates found.** A track or wheel part read its palette slot from the undecoded code, which drew a tank antenna as a black slab. It was fixed before the commit.
- **Unprimed critique.** An unprimed `screenshot-critique` over all 24 pairs and six crops found no missing, extra, misplaced or mis-posed unit. Its residue was low-confidence: a few dark speckle pixels on two distant tanks (market-town 382), and units hidden in the browser by effects (slice 17's).
- **Distance from the browser** (`compare-screenshots`), whole frames with effects and slice 15 included: mean 0.021 on market-town and 0.172 on paris-corner. Paris-corner is dominated by plate 1174's explosion, which fills the browser's frame.

**Cost.** Release export, market-town whole reel, units on against the same manifest with `models` dropped, three interleaved runs each, plugged in (`throwaway/slice14/bench/`):

| | p50 | p95 | p99 | Avg FPS | CPU p50 | Draws |
| --- | --- | --- | --- | --- | --- | --- |
| Units on | 27.12 ms | 39.15 | 43.06 | 36.2 | 15.16 | 1,594 |
| Units off | 25.00 ms | 37.25 | 39.73 | 39.0 | 13.79 | 1,232 |

The units layer costs about 2.1 ms at p50, 1.9 at p95 and 1.4 ms of CPU, plus 360 draws. Off run 2 found its window on screen, which is noted, not excluded.

**Against the browser, this bar fails.** Both columns are well over the browser's p95 of 23.9. The tree measured holds slice 15's static world, uncommitted, whose units-off CPU of 13.8 ms against slice 12's 1–2 ms carries most of the frame. The units' 1.9 ms at p95, on slice 12's 13.4, would keep native under the browser. The gate belongs to the combined tree, at slice 15's re-run and at slice 18.

**Not done.**
- **Impostor cards.** A model under `impostor_px` (10 px tall) draws its coarsest mesh tier; the browser draws a card. The report counts them (`units.impostors`): 279 over market-town's 18 plates, none on paris-corner. At that size the card and the 2,000-triangle tier are a few pixels of the same colours. See `choices.md`.
- **Side tint masks.** The surface takes a material's `tint` share uniformly, as slice 15's surface does. The browser's ORM alpha mask isn't ported.
