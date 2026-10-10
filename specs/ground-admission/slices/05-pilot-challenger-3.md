# 05 — Pilot: Challenger 3, and the two new profiles

**Unlocks:** the Challenger 3 card is buyable and fights. The
[admission recipe](../README.md#admission-recipe) is proven end to end and corrected. The
`light_tank` and `wheeled_tank_destroyer` profiles exist for slices 07–08.

**Why this card:** it shares the Challenger 2 TES frame and `challenger3.py` already draws
the `trophy_radar_L/R` / `trophy_launcher_L/R` nodes the `trophy_aps` part requires, so
every difficulty left is the recipe itself.

## Seam

- `fixtures/units/ground/profiles.json`: `roster_profile_light_tank`,
  `roster_profile_wheeled_tank_destroyer`, abstract, sensors and sound only, following the
  existing profiles. The light-tank rows `light_tank_ap`/`light_tank_he` already exist in
  `fixtures/game.json` (only supply prices name them today).
- `fixtures/units/roster/europe.json`: `europe_challenger_challenger_3` drops `planned`,
  `extends` the Challenger 2 TES record or `roster_profile_advanced_mbt` (choose the one
  that leaves fewer overrides; record it), `parts: ["trophy_aps"]`.
- Rename `packages/scene-assets/blender/roster/challenger3.py` → `challenger_3.py` to match
  its folder; switch it to `run`.
- `assets/catalog.json` appearance and wreck; remove the manifest entry; regenerate.

## Verification

- Catalog smoke test (`every_unit_type_sets_up_fires_each_mount_and_moves`) and the catalog
  gate (`the_browsers_catalog_view_is_current`), `crates/sim/tests/catalog.rs`.
- `asset validate --type europe_challenger_challenger_3` including `fit.part_nodes` (do the
  Trophy box names survive tiering? unverified), then `bake`, `check`, `icons`.
- Re-export: does a `run` export reproduce the `run_disabled` bytes? Record yes or no.
- Shot: `unit-roster` scene, Challenger 3 beside Challenger 2 TES; fix only what slice 01
  ranked for this card. [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md)
  against the Challenger 2 TES and the references, then an unprimed
  [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) as the last check.
- The card is buyable in a skirmish (lab or `/battle`).

Then rewrite the README's admission recipe with what differed, and time each step (export,
validate, bake) in [choices.md](../choices.md) so later slices can plan.

## Delegated

Armour, mobility and cost starting from the Challenger 2 TES and the KF51 (numbers are
guesses; the balance report at 13 tunes them); the profiles' sensor numbers, from the
nearest live profile.

## Stays green

Catalog tests; every existing digest (no rule changes here).

## Would change this slice

The recipe needing a shared-helper change. Then the coordinator owns that change and
re-exports every family using it, as a separate commit before this one.
