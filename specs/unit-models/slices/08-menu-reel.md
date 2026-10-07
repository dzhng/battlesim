# 08 The menu reel in roster looks, every shot the same

**Unlocks:** the main menu films roster-looking armies, and a test proves no
approved shot changed.

## The rule (user, 2026-10-06; write it into `fixtures/README.md`)

The menu backdrop is directed footage, not a battle. It may use **any units,
including menu-only units with whatever stats, hulls, weapons, scripts and
positions it needs**; it does not have to use real game stats, and anything in
it may change. The one requirement: **every approved shot stays exactly the
same**: camera, framing, timing, and what happens in it (who fires, which
round or missile flies, who is hit, who dies, when).

## Contract

- **Menu units** are `menu_*` cards in `fixtures/units/menu/`, resolved only
  into the menu's catalog (slice 04's third document set), never the game's,
  never purchasable. Each `extends` a test unit (slice 05) for behaviour and
  names the roster appearance it wears:

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

- **Hulls stay the test units'** (decided at implementation, 2026-10-07):
  with roster hulls the Market Town battle diverged from tick 219 (one shared
  scatter stream retimes everything after one changed shot), so no tuning of
  health, fire windows or positions could keep the shots. Menu units change
  only their look; the roster art overhangs its physical body (Abrams 0.47 m
  each end, HMMWV more), invisible at menu distance and accepted. Rejected:
  roster hulls (would need the Market Town scene refilmed).

## Work

1. **Record the approved reel first,** from the current build: for every shot
   in `fixtures/menu-backdrop.json`, keyframes (start, each event, end) and
   the event log of both encounters (`market-town/menu`, `paris-corner/corner`):
   ticks, shooter, round or missile, hit, death, and the framed subjects. Into
   scratch; this is the target.
2. **Exact-shot test (red first):** replay both menu encounters and compare
   their event log, under the unit mapping above, to the recorded target:
   same events, same order, same ticks, every framed subject alive while
   framed. Digests can't be the gate (the ids change); this test is. Add it to
   the suite: no test pins the menu encounters today, and a `follow` on a dead
   unit silently frames where it was (`MenuBackdrop.tsx:171-175`).
3. Menu units and both encounters switched; tune behaviour (never the shot
   list) until the test passes.
4. Compare every shot's keyframes against the target with compare-screenshots:
   framing and action identical, only the units' look changed.
5. Write the rule into `fixtures/README.md`'s menu backdrop paragraph.

## Verify

The exact-shot test; compare-screenshots per shot; screenshot-critique
unprimed, last; preview-shots for the user. Non-blocking: about five minutes,
then decide on the evidence, record it in choices.md, close the shots,
continue. The menu catalog's resolution is covered by slice 04's tests.

## Stays green

The shot list in `fixtures/menu-backdrop.json`, untouched.

## Delegated

Menu-unit stats, positions, scripts and fire windows, as long as the
exact-shot test and the keyframe comparison pass.
