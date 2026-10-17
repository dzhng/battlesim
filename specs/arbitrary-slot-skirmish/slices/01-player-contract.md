# Slice 01: one player contract, mechanical port

**Decisions:** D29, D32, D33, D34, D35, L8, L11.

## Contract it unlocks

The contracts in the README, with exactly one player per side. The shape changes; behaviour doesn't.

## Seam

- **`crates/contract/src/players.rs`** (new): `PlayerSlot`, with `faction` required; `PlayerIndex`; `entry_count`; player-list validation.
- **`ScenarioDefinition.players`** is the single list. `UnitSetup.owner` names a player id.
- **Bundles.** `bundle.rs` drops its own `slots` list and validates against `scenario.players`. Economy bundles derive the skirmish from those players. Bump `scenario-bundle-1`.
- **`SkirmishSetup`** becomes `{ rules, sites }`. `PrepareBattleRequest.factions` becomes `players`.
- **`SkirmishSetup::side_faction(side)`** is the one place that answers "this side's faction". It replaces `factions[i]` in the simulation, the AI and `presentation/src/fielded.rs:87`.
- **Rename** `occupied_slots` to `unit_count` everywhere.
- **Fixture:** `fixtures/skirmish.json` gains `max_players_per_side: 10`.
- **Short-lived seam:** the simulation keeps its side arrays, mapped from the one player per side. Slice 02 deletes this mapping.
- **Browser:** sends players `[blue-1: Human, red-1: active AI]`, built from today's faction and enemy choices. Slice 06 replaces this.

## What the human can run or see

A 1v1 generated battle plays exactly as before.

## Red tests first

- **Player-list JSON** is refused for: an empty side, a duplicate id, mixed factions on one side, a Neutral skirmish player, 11 per side, and a wrong entry count. An all-AI 5v5 and a 10v10 are accepted.
- **Request JSON** round-trips canonically, and a `factions`-shaped request is refused.
- **Bundle round trip:** an Economy bundle's players become its skirmish players.

## Verification

Every digest is unchanged: skirmish and non-skirmish, the parity fixtures, the codec vectors. That proves the change was only a change of shape.

## Delegated

Module-internal layout, error message wording, how the TypeScript mirror type is generated or hand-written.

## Stays green

All suites. Every digest.

## Feedback that would change it

None expected; this is a mechanical port.
