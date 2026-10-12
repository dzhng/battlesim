# 07 — Tactical canvas authoring and explicit file I/O

## Contract unlocked

A developer can create/edit the common scenario fields on a map canvas, validate them through the canonical bundle contract, and Download/Upload without hidden persistence.

## Seam

Add a dedicated scenario-builder route/module under `apps/battle-lab` or a first-class app owner chosen by repository shape. Reuse existing map geometry, camera, picking, catalog, and battle presentation owners. The editor state maps one-to-one to `ScenarioBundle`; advanced script source is opaque text validated by slice 05, not a second UI schema.

The common UI covers resolved map snapshot import/preview, player slots, side/faction/controller fields, unit palette and placement, unit inspector, sandbox/economy mode basics, common victory/event fields, and user asset upload. Download creates the self-contained package. Upload validates atomically and previews it in memory.

## Runnable artifact

The route starts from a tiny deterministic in-memory sample, lets the user add/select/move units, changes slot/controller fields, downloads a package, and uploads it back into a fresh draft. No codebase file or browser storage is written.

## Visual gates

Load `game-ui` before implementation. Split visual review by one variable per shot:

1. tactical map and placement readability (canvas crop);
2. unit/slot markers and selected-unit inspector (right-panel crop);
3. validation, upload, and asset refusal legibility (status-panel crop).

For each shot run `compare-screenshots` when a target exists, then unprimed `screenshot-critique` last. Open shots with `preview-shots` for a non-blocking review window and record the decision if no response arrives.

## Must stay green

Bundle round-trip, map snapshot, unit catalog, and browser admission tests. Do not make the editor depend on current fixture paths.

