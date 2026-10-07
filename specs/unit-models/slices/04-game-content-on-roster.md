# 04 Game content on the roster

**Unlocks:** a request the game can't honour is refused rather than
defaulted, and the menu reel shows roster vehicles and soldiers.

Tests are not game content: they stay on test units (slice 05).

## A battle request names both factions or is refused

`/battle` without `faction` is an invalid address: `apps/battle-lab/src/battleLinks.ts`
returns an error naming the missing field, as it does for `map`, and unknown
parameters are refused too (today `askedBattle` ignores them, so old
`recipe=` links pass silently). The faction-less preparation branch in
`web/src/battle/prepare/prepare.ts` (the recipe-planned battle) is deleted.
`recipe_id` and `encounter_seed` leave `PrepareBattleRequest`
(`crates/contract/src/preparation.rs`, `deny_unknown_fields`) and
`preparedBattleHref` stops writing them (decision 9): a contract change, so the
engine id moves and saved replays are refused, accepted with no compatibility.
Ordinary Play already sends factions.

Who still plans recipes, for the record: the map workbench and its report
(`apps/map-workbench/server.ts`, `crates/mapgen/examples/map_workbench_report.rs`)
and tests. The benchmark and the endurance lab do **not**: they use the
synthetic `city-arena-2` stress scene with an army hard-coded in
`crates/sim/src/endurance.rs` (slice 05 renames it to test units).
`fixtures/encounters.json` `assault` names test units; it stays only while
those callers do, and `generated-battle.json` stops naming a default recipe.

## The menu reel is a film (user, 2026-10-06)

The menu backdrop is directed footage, not a battle. It may use **any units,
including menu-only units with whatever stats, hulls, weapons, scripts and
positions it needs**; it does not have to use real game stats, and anything in
it may change. The one requirement: **every approved shot stays exactly the
same** (camera, framing, timing, and what happens in it: who fires, which
round or missile flies, who is hit, who dies, when). Write this rule into
`fixtures/README.md`'s menu backdrop paragraph.

What changes is how the units look: blue looks like the US roster, red like
the Eastern roster, per the approved mapping:

| Side | Was | Looks like |
|---|---|---|
| blue | `tank` | `us_m1_abrams_sep_v2` |
| blue | `jeep` | `us_m1151_hmmwv_hmg` |
| blue | `rifle` | `rifle_squad` |
| blue | `at` | `us_atgm_team_bgm_71_tow_2a` |
| blue | `recon` | `us_army_scouts_light_patrol` |
| red | `rifle` | `rifle_squad` |
| red | `at` | `eastern_atgm_team_kornet` (guided, like the original) |
| red | `tank` | `eastern_t_72_t_72b3_2016` |

**How:** menu-only unit types in a menu catalog document (resolved for the
menu session only, like the test catalog, slice 05), each wearing the roster
unit's appearance and carrying whatever behaviour reproduces the shot: usually
the generic unit's stats and weapons, with hull sizes and positions adjusted
where the roster art is a different size (the Abrams is 0.47 m longer than the
generic tank; `paris-corner` tuned the tank to pass 1.85 m from the infantry,
`1296510d`). Menu-only types are menu content: named `menu_*`, never in the
game catalog, never purchasable.

## Work

1. **Test first:** `/battle` without a faction, and with an unknown
   parameter, is refused by name (`web/tests` for `battleLinks`); red today.
   Delete the branch and contract fields.
2. **Record the approved reel before touching it:** for every shot in
   `fixtures/menu-backdrop.json`, capture keyframes (start, each event, end)
   and the event log (fires, hits, deaths with times and unit ids) from the
   current build, into scratch. This is the target.
3. Menu-only types and the two encounters (`market-town/menu`,
   `paris-corner/corner`) switched to them.
4. **Exact-shot check:** a test replays both menu encounters and compares the
   event log to the recorded target: same events, same order, same ticks, same
   shot subjects alive while framed. Then compare every shot's keyframes
   against the target with compare-screenshots: framing and action identical,
   only the units' look changed. Iterate on menu-unit stats, hulls, positions
   and scripts (never the shot list) until both pass.
5. `fixtures/README.md` gets the menu-reel rule.

## Verify

The refusal test; the exact-shot test; compare-screenshots per shot;
screenshot-critique unprimed, last; preview-shots for the user. Non-blocking:
about five minutes, then decide on the evidence, record it in choices.md,
close the shots, continue.

## Delegated

Menu-unit stats, hulls, positions and scripts, as long as every shot matches.
