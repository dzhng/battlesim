# 01 Presentation by property

**Unlocks:** every roster vehicle sounds and articulates as its kind, and a new
roster unit needs no presentation table edit.

## Contract

The ground profile (`fixtures/units/ground/profiles.json`, `roster_profile_*`)
gains `presentation.vehicle` naming a vehicle presentation class:
`heavy_tracked`, `light_tracked`, `heavy_wheeled`, `light_wheeled`, `truck`.
`fixtures/game.json`'s presentation `sound.vehicles` and `pose.gauge` rows are
keyed by that class instead of a unit id; `fixtures/sounds.json` `units` shot
overrides are keyed by weapon id (a mount's weapon) rather than unit id. The
observation or catalog already tells the client each unit's type; the client
reads its class from the resolved catalog.

## Work

1. **Test first** (`write-tests`): for every roster vehicle card, the
   resolved presentation class exists and the sound frame picks a non-default
   vehicle row and gauge. Red today.
2. Add the field to each vehicle profile; rekey the three tables; change the
   two readers (`soundFrame.ts` `pick`, `poseDriver.ts` `halfTrack`) and the
   shot-sound lookup in `battle-audio` `catalog.ts`.
3. **Reuse the generic rows, don't re-author them.** The tuned generic sounds
   become the classes' sounds: `tank`'s row → `heavy_tracked` (and, until
   tuned, `light_tracked`), `jeep`'s → `light_wheeled`, `supply`'s → `truck`
   (and, until tuned, `heavy_wheeled`); gauge likewise. Per-unit shot
   overrides of `tank`, `jeep`, `rifle`, `recon`, `at` move to the weapon ids
   those units fire, so roster units firing the same weapon get them.
   Stand-ins (still `tank`, `jeep`, `supply` until slice 05) take the same
   classes, so tests keep their sounds.

## Verify

The test above; `battle-audio` and `battle-renderer` unit tests for the two
readers. No battle run: presentation only. Listen once in a generated battle
with an Abrams, a Stryker and a HEMTT (sound workbench or a skirmish).

## Delegated

Class names; where the client reads the class from.
