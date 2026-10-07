# 07 Battle request names both factions

**Unlocks:** a battle request the game can't honour is refused by name, never
defaulted.

## Contract

- `PrepareBattleRequest` (`crates/contract/src/preparation.rs:14-30`,
  `deny_unknown_fields`) becomes `{ map_source, factions: [Faction; 2],
  battle_seed }`: `recipe_id` and `encounter_seed` leave it, factions are
  required. A contract change: the engine id moves and saved replays are
  refused, accepted with no compatibility.
- `/battle` (`apps/battle-lab/src/battleLinks.ts`) refuses a missing faction
  and any unknown parameter by name, as it refuses `map` today. Today factions
  are optional (`AskedBattle`, `:26-46`; parsing checks them only when present,
  `:131-140`) and unknown parameters are ignored, so old `recipe=` links pass.
  `preparedBattleHref` stops writing `recipe` and `encounter`.
- The faction-less preparation branch in `web/src/battle/prepare/prepare.ts`
  (the recipe-planned battle) is deleted. Ordinary Play already sends factions.
- Encounter recipes leave player preparation. They stay only where a tool
  plans one on a generated map to judge it: the map workbench and its report
  (`apps/map-workbench/server.ts`, `crates/mapgen/examples/map_workbench_report.rs`)
  and tests, naming test units (slice 05). The benchmark and the endurance lab
  never planned recipes (they run the synthetic `city-arena-2` stress scene,
  `benchmark/prepare.ts:22-34`, `endurance.tsx:84-102`).
  `generated-battle.json` stops naming a default recipe.

## Work

1. **Tests first:** `crates/contract/tests/preparation.rs` refuses a request
   without factions and with the removed fields; `battleLinks` tests refuse a
   missing faction and an unknown parameter by name. Red today.
2. Contract, links and preparation changed; the map workbench passes its
   recipe to the planner directly, not through the player request.

## Verify

The tests; `web/tests/prepareBattle.test.ts`, `skirmishPreparation.test.ts`;
one Play scene.

## Delegated

Error message wording.
