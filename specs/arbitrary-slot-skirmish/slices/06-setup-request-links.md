# Slice 06: one setup constructor, request and links

**Decisions:** D34, D38, D42, L10. **Needs:** 03 and 05.

## Contract it unlocks

Any team battle can be launched by link: from the lobby later, and from the benchmark and the tournament now.

## Seam

- **`sim::skirmish::setup_for(players, sites)`** replaces three constructors:
  - `game-wasm/src/lib.rs:991` (`skirmish_fields` is deleted);
  - `skirmish_tournament/inputs.rs:126`;
  - the builder's `builderEconomy` (`apps/battle-lab/src/scenario-builder/model.ts:80`).
- **Synthetic entry layouts** (`inputs.rs:145`, `model.ts:80`) get one owner, built from `entry_count`.
- **Request.** TypeScript `PrepareBattleRequest` (`web/src/battle/prepare/protocol.ts:14`) carries `players`. `prepare.ts` stops splicing JSON strings and points the camera at the human's lane, or at Blue's centre when no human plays (D34).
- **Links.** `battleLinks.ts` gains a player-list grammar: per side, in order, each slot's controller, plus each side's faction.
  - The benchmark (`apps/battle-lab/src/benchmark/prepare.ts`) and the endurance route build players.
  - The single-player link requires exactly one Human, on Blue.
  - An old-shape battle link is refused, naming the problem (D42).

## What the human can run or see

A hand-typed 3v3 link, and a 5v5 all-AI benchmark link, both play.

## Red tests first

- Golden tests for request JSON at 1v1, 3v4 and 5v5.
- Link table tests: round trips; an old `enemy=` link refused as unknown; an old link without `enemy` refused as missing players; a single-player link with zero or two humans refused.
- The tournament and the builder produce the same setup as `setup_for`.

## Delegated

The exact link spelling, within "readable and canonical". Error wording.

## Stays green

Every digest from slice 05.

## Feedback that would change it

The user wanting shorter shareable links.
