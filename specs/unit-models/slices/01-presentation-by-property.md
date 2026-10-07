# 01 Presentation by property

**Unlocks:** every roster vehicle sounds and articulates as its kind, every
roster weapon gets its tuned shot, tracer, flash and missile motor, and a new
roster unit needs no presentation table edit.

## What is wrong

- Vehicle engine, track and turret loops (`fixtures/game.json` presentation
  sound `vehicles`, `packages/battle-audio/src/soundFrame.ts:695`) and the
  pose gauge (`presentation.pose.gauge`, `game.json:1074`,
  `packages/battle-renderer/src/models/poseDriver.ts:434`) are keyed by unit
  id `tank`/`jeep`/`supply`: every roster vehicle gets the default row.
- Weapon tables: since `6c258b3d`, a derived roster weapon (`ground_tank_ap`,
  `tow`, `kornet`, `assault_rifle`, …) takes its nearest ancestor's `shots`,
  `impact_scale` and `blasts` rows and its effects (`inheritRows` in
  `soundFeed.ts`, `effectFeed.ts`). `motors` is not inherited, so TOW and
  Kornet missiles still have no motor loop: add it to the same `inheritRows`
  call.
- `fixtures/sounds.json` `units` overrides are keyed by unit and mount.

## Contract (decision 6)

- **Vehicle class is derived from physics, not authored:** mobility (tracked
  or wheeled) × hull `weight_class`, with the `logistics` role splitting
  trucks. Today's hulls fall into tracked heavy (16), tracked medium (7),
  wheeled light (4), wheeled medium (14), wheeled medium logistics (4). No
  new catalog field, so `UnitType` (`deny_unknown_fields`,
  `crates/contract/src/catalog.rs:100`) and the engine id don't change.
- **Weapon rows: done on main** (`6c258b3d`, 2026-10-06): a weapon without
  a row takes its nearest ancestor's through `extends`
  (`renderer-core/src/kindTable.ts` `inheritRows`, used by `effectFeed.ts` and
  `soundFeed.ts`), so an assault rifle looks and sounds like the rifle it
  extends; tested on fake weapons (`web/tests/kindTable.test.ts`). That is the
  one owner for weapon presentation; don't add a second (the earlier plan's
  published `base` field is dropped). What remains here is `motors`, and the
  unit-keyed `sounds.json` `units` overrides.
- **One TypeScript owner** for the vehicle class: `vehicleClass(type)` in
  `packages/scene-assets/src/units.ts`, already "the one owner on the
  TypeScript side" of derived unit questions (`units.ts:1-6`). The same class
  keys the unit budgets (slice 12); no second class table anywhere.
- **Overrides fold into `sounds.json` `defaults`** (already per weapon,
  validated against `presentation.audio.shots` by
  `apps/sound-workbench/server.ts:161`). Two collisions are decided here:
  HMG → `hmg-combat` (the jeep's), and rifle → `rifle-combat` (recon's
  `recon-rifle` goes); record both in choices.md.

## Reuse the generic rows, don't re-author them

The tuned generic sounds become the classes': `tank`'s row → tracked heavy and
tracked medium, `jeep`'s → wheeled light, `supply`'s → wheeled medium
logistics and wheeled medium; gauge likewise.

## Work

1. **Test first** (`write-tests`): every roster vehicle card resolves to a
   non-default vehicle sound and gauge, and a guided missile that extends one
   with a motor row gets that motor (on fake weapons, like
   `web/tests/kindTable.test.ts`). Red today.
2. Classes and keys; rekey the tables; change the readers (`soundFrame.ts`,
   `poseDriver.ts`, the effect and audio table lookups); fold the overrides.
3. The sound workbench (`apps/sound-workbench`), which is built around unit ×
   mount and checks ids against `fixtures/catalog.json` (`server.ts:164-168`),
   edits weapon rows and vehicle classes instead.
4. Test units (still `tank`, `jeep`, `supply` until slice 05) derive their
   classes the same way, so tests keep their sounds.

## Verify

The test; `battle-audio`, `battle-renderer` and sound workbench tests. No
battle run: presentation only. Listen once in a skirmish with an Abrams, a
Stryker, a HEMTT and a TOW team.

## Delegated

Class names.
