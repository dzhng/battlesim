# Team skirmish

**Status:** in progress (slices 00–02 and 07 done).
**Updated:** 2026-10-17.

## Next Agent Prompt

**Done:** slices 00, 01, 02 and 07.
- Slice 00's probe changed the plan: both sides get the same number of entries (D52), and arm spacing and river avoidance are hard tests (D53).
- Slice 01 landed the player contract. D54 makes Neutral players factionless.
- Slice 02 made every player a full player in the simulation and removed slice 01's temporary seam. It re-baselined the skirmish digests (D41, re-baseline 1); the before and after values are in its commit message.
- Slice 07 landed the pure lobby model and moved the builder onto `scenario.players`.

**Next:**
- Slice 03 (publication): the side view still shows one player's economy.
- Resume slice 04 (entry arms). Its Codex session was paused for disk space in worktree `team-entries`, branch `team-skirmish/team-entries`.
- 05 waits on 02 and 04.

**Known environment failures.** A worktree needs `git lfs pull --include 'assets/runtime,assets/third-party,packages/scene-assets,web/scenes'` before browser tests and scenes, which is about 0.75 GB. After that, only these fail, identically on main, because they need LFS source art:
- `icons.test.ts` (3 tests);
- `soundCatalog.test.ts`.

`battleFailure.test.ts` is flaky under heavy machine load: the headless browser's no-WebGPU error races the replay refusal. Fix it in its own commit.

Before touching code, read three things:
- this README;
- [`choices.md`](choices.md), which records every decision with who made it and why;
- the slice file.

Every choice is closed. Reopen one only by asking the user. Decisions marked "amended" are superseded by the decision they name.

How to work each slice:
- Write red tests first.
- Keep a 1v1 playable in the browser after every slice.
- Re-baseline digests only where D41 names it.
- On visual slices, run the visual gates, with `screenshot-critique` last.
- Before you end a pass, tick the slice below and rewrite this prompt for whoever comes next.

Slices:

- [x] 00: probe multi-entry admission, timings, throughput ([slice](slices/00-probe.md))
- [x] 01: one player contract, mechanical port ([slice](slices/01-player-contract.md))
- [x] 02: simulation per-player ownership ([slice](slices/02-sim-ownership.md))
- [ ] 03: publication and TypeScript mirror ([slice](slices/03-publication.md))
- [ ] 04: mapgen entry arms and admission ([slice](slices/04-entry-arms.md))
- [ ] 05: simulation entry owner and dispatch ([slice](slices/05-entry-owner.md))
- [ ] 06: one setup constructor, request and links ([slice](slices/06-setup-request-links.md))
- [x] 07: lobby model ([slice](slices/07-lobby-model.md))
- [ ] 08: lobby map picture ([slice](slices/08-map-picture.md))
- [ ] 09: order-of-battle lobby screen ([slice](slices/09-lobby-screen.md))
- [ ] 10: allies in battle ([slice](slices/10-allies-in-battle.md))
- [ ] 11: per-player match report ([slice](slices/11-match-report.md))
- [ ] 12: team planner, lanes and cohesion ([slice](slices/12-team-lanes.md))
- [ ] 13: requests for help and shared stance ([slice](slices/13-team-support.md))
- [ ] 14: 5v5 AI-versus-AI closeout ([slice](slices/14-closeout.md))

## Goal

Team skirmish from 1v1 to 5v5, using one general system. A 1v1 is one player per side, with no special case. Each player has its own:
- wallet;
- 30-unit cap;
- army and logistics;
- ready flag;
- controller.

Allies share vision, objectives, score and a bounty pool.

The lobby is a desktop order-of-battle screen. Blue's player banners sit on the left, a real generated map in the centre, and Red's banners on the right. Presets seed 1v1, 3v3 and 5v5, and Add/Delete makes uneven matches. Single-player has exactly one Human, on Blue. AI players are full combined-arms armies. A per-side team planner coordinates them: lanes, staging, requests for help, and a shared stance.

The end state reads as if team play had been designed in from the start: one player list, one entry owner, one setup constructor, one selection rule. No two-faction leftovers.

Not in this plan:
- online play (the model is ready for it: D26);
- mixed factions within a side;
- mobile layout;
- native lobby and HUD ([native gaps](../native-gaps/README.md), item 10).

There is no backward compatibility and no migration.

## Contracts

Changing any of these is a named decision in `choices.md`.

**Terminology.** A *player* is one seat on a side: its own wallet, cap, army, ready flag and controller. The code calls it `Player` too (D51). Slice 01 renames `PlayerSlot`, `CommandEnvelope.slot`, `AuthoredScript.slot` and `occupied_slots`, so the word "slot" keeps only its unrelated meanings (squad member and garrison seat).

- **Players (`contract::players`).** `Player { id, side, faction, controller, human_capable }`. `side` is Blue or Red (Neutral only in authored bundles). `faction` is required for Blue and Red, and absent for Neutral (D54).
  - `ScenarioDefinition.players` is the only player list (D33).
  - `PlayerIndex` is a player's position in that list. It is derived, never stored (D32).
  - `entry_count(players)` gives 1, 2 or 3 entries. Both sides get `entry_count` of the larger side's player count (D52). It is the one rule that both mapgen and the simulation read.
- **`SkirmishSetup`** is `{ rules, sites }`; the players live in the scenario.
  - Admission refuses a side with no players, duplicate ids, mixed factions within a side, more than `max_players_per_side` (D35), or an entry count other than `entry_count`.
  - A setup with zero humans is valid (D34).
- **`SkirmishSites.entries`** is a list of 1–3 entries per side, the same count on both sides (D52), ordered centre, left, right. There is no player-to-entry reference.
- **Generation request** carries the entry count per side. That count is part of map identity, so the generator version is bumped. A 1-entry request generates today's map, byte for byte.
- **`PrepareBattleRequest`** is `{ map_source, players, battle_seed }`.
- **Commands.**
  - In a skirmish, `CommandEnvelope.player` is required.
  - `seq` and the order rate limit are counted per player.
  - Ready, purchase and cancel orders are checked against the player.
  - Purchase IDs are scoped to the player.
- **Observation.** Still one per side.
  - The skirmish view carries `players` with each own-side player's credits, unit count, cap, pending purchases and ready flag (D31).
  - `OwnUnit.owner` is a `PlayerIndex`.
  - Report totals and events are per player. Scores stay per side.
- **Units.** A bought unit spawns owned by the player that bought it. A pending purchase stores the entry it will arrive at (D37). Death provenance records the killer's player. `owner` is hashed into the unit digest (D44).
- **URL.** A battle link carries the map choice, the battle seed and the player list. Anything else is refused (D42).

## Single owners

| Concept | Owner | Replaces |
|---|---|---|
| Player list and validation | `crates/contract/src/players.rs` (new) | `bundle.rs` players, `factions: [_; 2]`, `ai_side` |
| Entry count rule | `contract::players::entry_count` | (new) |
| Entry geometry | `crates/mapgen/src/skirmish.rs`, its own `"entries"` stream | "first stroke on the edge" |
| Map fairness admission | `crates/sim/src/map_analysis.rs` | two-entry `as_chunks::<2>` |
| Arrival and return entry | `crates/sim/src/entries.rs` (new) | six `find(side)` sites and `runner.rs:98` |
| Setup construction | `sim::skirmish::setup_for` | three hand-written constructors (D38) |
| Economy, controllers, AI dispatch, settlement | `crates/sim` | side arrays |
| Controller policy | one function in `battle.rs` (D45) | `controller_turn` and `BalancedDoctrine` both driving the same player |
| Team planner | `crates/sim/src/skirmish_ai/team.rs` (new) | (new) |
| Map picture | `mapgen::inspect`, from a `MapDefinition`, exposed through wasm (D39) | (new) |
| Lobby state, URL identity | `apps/battle-lab/src/lobby/` (new), `battleLinks.ts` | the builder's `BuilderSlot` duplicate |
| Preview lifecycle | a preview controller over `web/src/battle/prepare/admission.ts` | (new) |
| "Which units can I order" | `controlled(observation, player)` in web (D47) | five ad-hoc `observation.own` filters |

**One short-lived seam (slice 01 → 02).** Slice 01 keeps every digest unchanged with four temporary pieces. Slice 02 removes all four at its named re-baseline:
- the simulation keeps its side-indexed internals, mapped from the single player per side;
- `scenario_digest` hashes the old `{factions, rules, sites}` projection;
- `MatchRules.max_players_per_side` is omitted from JSON when it is 10;
- scenario validation refuses more than one player per side ("one player per side until per-player ownership lands"), since the simulation still has one AI and one wallet per side.

Nothing else is transitional.

## Invariants

- One player type, one player list. A second player concept anywhere is a defect.
- Every skirmish unit has an owner, and every skirmish command carries a player. A refused AI order fails a debug assertion and appears in `ai_receipts()`.
- Vision, objectives, score and the bounty pool belong to the side. Wallet, cap, ready, purchases, controller and AI belong to the player.
- Every ground or helicopter arrival, refund, fuel return and retirement uses `sim::entries`: the side's entry with the shortest drive for that unit type, ties to the centre. Jets arrive opposite the centre entry.
- The team plan comes from the side's observation plus non-digested planner memory, evaluated in player order. The command stream is the replay authority.
- Digests follow D41: slice 01 changes none, and non-skirmish digests never change.
- The preview is deterministic. Deploy re-prepares the admitted request and gets the same battle. A newer preview always cancels an older one.

## Slice graph

```
00 probe ─┬─► 01 contract ─► 02 ownership ─┬─► 03 publication ─┬─► 06 setup/request/links ─► 07 lobby model ─► 08 map picture ─► 09 lobby screen
          │                                │                    └─► 10 allies ─► 11 report
          └─► 04 entry arms ───────────────┴─► 05 entry owner ─► 12 team lanes ─► 13 team support ─► 14 closeout (needs all)
```

- **05** needs both 02 and 04.
- **06** needs 03 and 05.
- **10** needs 06, so a team battle can be launched by link.
- **07** may start once 01 lands; it only needs the TypeScript mirror of the player type.
- **08** needs 04 (multi-entry sites to draw) as well as 07.
- **12** needs 05 (lanes come from entries) and 03 (the planner reads `OwnUnit.owner` and `goal`).

**Visual slices** are 08, 09, 10 and 11. Each judges one visual variable:
- 08: map legibility at lobby size;
- 09: lobby composition and state clarity;
- 10: telling allies from your own units;
- 11: report readability.

Each uses `game-ui` before building, then:
1. `compare-screenshots` against its named target;
2. `screenshot-critique`, unprimed, as the **last** check before acceptance;
3. a non-blocking `preview-shots` review. Give the user about 5 minutes; if they're silent, decide on the evidence, record why in `choices.md`, close the shots and continue.

## Browser checkpoints

- **After 01:** a 1v1 generated battle plays, with every digest unchanged.
- **After 02:** the 1v1 HUD shows the human player's own credits and cap.
- **After 05:** a 3v4 all-AI benchmark battle shows units arriving at the entry nearest their order.
- **After 09:** the lobby scene covers:
  - the 1v1 default and the 3v3 and 5v5 presets;
  - 3v4 and 2v3 via Add/Delete;
  - human transfer and the size ratchet;
  - preview on open and on change, Reroll, and a hidden retry;
  - Deploy reproducing the exact map.
- **After 10–11:** a 2v2 with you and one AI ally covers the ally look, the read-only ally panel, own-only selection and the per-player report.
- **At 14:** the 5v5 AI-versus-AI runs.

## Research

The team AI design draws on the Killzone 3 commander, Kohan II's goal pricing, Age of Empires II team scripts and UAlbertaBot's hysteresis. See [research.md](research.md).

## Superseded artifacts

- [`visualizations/unknowns-map.html`](visualizations/unknowns-map.html) records the planning walk. Its build order and implementation prompt are superseded by this README.
- [`visualizations/lobby-directions.html`](visualizations/lobby-directions.html) is the lobby mock. Direction 3 is the chosen target (D22).
