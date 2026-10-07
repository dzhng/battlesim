# 04 Game content on the roster

**Unlocks:** what a player can reach shows only roster units, and a request
the game can't honour is refused rather than defaulted.

Tests are not game content: they stay on stand-ins (slice 05).

## Contract

**A battle request names both factions or is refused.** `/battle` without
`faction` is an invalid address: `apps/battle-lab/src/battleLinks.ts` returns
an error naming the missing field, as it already does for `map`. The
faction-less preparation branch in `web/src/battle/prepare/prepare.ts` (the
recipe-planned battle) and the `/battle` `recipe` and `encounter` parameters
are deleted. Ordinary Play, the only player route, already sends factions and
is untouched. Shared addresses without a faction stop working, as intended.

**The menu backdrop films the roster** (user, 2026-10-06): as close to the
original as possible in unit type and scenes; blue is US, the enemy is
Eastern. Mapping (approved):

| Side | Was | Becomes |
|---|---|---|
| blue | `tank` | `us_m1_abrams_sep_v2` (not a Trophy variant: its APS would defeat the missile hits the reel is directed around) |
| blue | `jeep` | `us_m1151_hmmwv_hmg` |
| blue | `rifle` | `rifle_squad` (US) |
| blue | `at` | `us_atgm_team_bgm_71_tow_2a` |
| blue | `recon` | `us_army_scouts_light_patrol` |
| red | `rifle` | `rifle_squad` (Eastern) |
| red | `at` | `eastern_rpg_team_rpg_7` |
| red | `tank` | `eastern_t_72_t_72b3_2016` |

Same positions, scripts and shots; the scenes' factions become US and Eastern.

**Developer tools keep stand-ins, explicitly.** The benchmark, the endurance
lab and the map workbench plan encounters from `fixtures/encounters.json` to
measure frame cost or judge a map. A roster change must not move a benchmark
baseline or a map verdict, so their recipes name stand-ins (after slice 05,
`stand_in_*`) and each tool's readme says so. A recipe with no remaining
caller is deleted.

## Work

1. **Test first:** `/battle` without a faction is refused with a named error
   (`web/tests` for `battleLinks`); red today. Delete the faction-less branch
   and parameters.
2. Menu backdrop armies to roster ids. Then redirect the reel: keep each liked
   shot; retime with starting health and fire windows (`fixtures/README.md`),
   not by re-cutting.
3. Endurance (`crates/sim/src/endurance.rs` `ROSTER`, a misnomer) is renamed to
   say it is a stand-in army; nothing else changes there.
4. Remove recipes and parameters nothing calls.

## Verify

The refusal test. The menu reel: record before and after, compare shot by shot
(compare-screenshots), screenshot-critique unprimed, last, then show the user
with preview-shots. Non-blocking: about five minutes, then decide on the
evidence, record it in choices.md, close the shots, continue. The two menu
encounters' digests move; regenerate any parity that pins them and name it in
choices.md.

## Delegated

Retiming values (starting health, fire windows) that keep the liked shots.
