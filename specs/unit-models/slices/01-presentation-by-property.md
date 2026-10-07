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
- Weapon-keyed tables (audio `shots`, `impact_scale`, `motors`; effects
  `tracers`, `flashes`, `impact_scale`; looked up exactly by
  `renderer-core/src/kindTable.ts` `pick`) are keyed by base weapon ids
  (`rifle`, `hmg`, `tank_ap`, `tank_he`, `atgm`, `grenade`), but roster units
  fire derived rows (`ground_tank_ap`, `tow`, `kornet`, `rpg_light`,
  `assault_rifle`, `autocannon`, `ifv_he`), and the resolved weapon view drops
  `extends`. So every roster shot is the default, and TOW and Kornet missiles
  have no motor loop.
- `fixtures/sounds.json` `units` overrides are keyed by unit and mount.

## Contract (decision 6)

- **Vehicle class is derived from physics, not authored:** mobility (tracked
  or wheeled) × hull `weight_class`, with the `logistics` role splitting
  trucks. Today's hulls fall into tracked heavy (15), tracked medium (7),
  wheeled light (4), wheeled medium (13), wheeled medium logistics (4). No
  new catalog field, so `UnitType` (`deny_unknown_fields`,
  `crates/contract/src/catalog.rs:100`) and the engine id don't change.
- **Weapon presentation key:** each resolved weapon carries the root of its
  `extends` chain (its base weapon) as its presentation key; tables are
  looked up by key. Whether that is a resolved field or derived client-side is
  the implementer's choice, provided it doesn't change `config_digest`
  (`battle.rs:505`) or the engine id; if it must, say so and name the move.
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
   non-default vehicle sound and gauge, and every roster weapon to a
   non-default shot, tracer and flash (and motor for guided missiles). Red today.
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

Class names; where the presentation key is computed (within the digest rule).
