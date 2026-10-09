# 02 Wreck debris scatters and fades

**Unlocks:** a dead vehicle throws armour packs, doors, track runs and
stowage several metres around it, as a war film would show, and that scatter
disappears after a while, so it never reads as cover the simulation lacks.

User, 2026-10-08: "May wreck debris scatter wider than the hull - yes, maybe
debris that scattered can just disappear?"

## Contract

- **Simulation unchanged.** The wreck prop, its physical box and its cover
  stay as they are (`Hull.wreck`, `wreck_of`); no digest moves.
- **Art:** a wreck appearance may carry debris nodes named `debris_*`. The
  wreck footprint check (`packages/scene-assets/src/validate.ts`,
  `footprintFindings`) leaves them out and holds them to their own loose
  allowance (a scatter radius from the hull, recorded in choices.md, a
  tripwire not a target). `wreckage.burn()` keeps `debris_*` nodes (it
  deletes `dressing_*`).
- **Renderer:** debris draws with the wreck when it appears, then fades out
  over a few seconds after a hold (presentation constants in `fixtures/game.json`
  presentation, loose). One owner for the fade in the battle renderer's wreck
  / cook-off path; the wreck body itself never fades. Load the `renderer`
  skill first.
- **Families:** add debris to every vehicle family's wreck through one shared
  helper in `wreckage.py` (thrown armour packs, doors, track runs, crates,
  wheels for wheeled vehicles, from what each family carries), so no family
  hand-places debris. Re-export every roster vehicle's wreck (byte-identical twice), bake; the disabled lanes (slices 03, 04) re-export their own through the helper after this merges.

## Verify

Tests first: a `debris_*` node outside the footprint passes the wreck check
and one past the allowance fails; `burn()` keeps debris; the renderer fades
debris and not the hull (unit test on the fade owner). A cook-off lab scene
with a tracked and a wheeled vehicle: screenshots at the death, after the
hold, after the fade; compare-screenshots; critique last.
