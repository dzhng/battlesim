# 08 Walkthrough harness, chapters 1–2 directed

**Unlocks:** the standing regression gate for every scenario, proved on chapters
1–2.

**Contract (C8):** `fixtures/maps/<id>/scenarios/<name>.solution.json` holds
director beats (06 vocabulary) that order the **human** player's units: buy,
place, ready, move, set engagement. Targets are chosen only through `Knows`
triggers, so the solution attacks only what blue has identified.
`ScenarioDocument::with_solution(sol)` merges it into the director under the
same admission and puts the human seat under `DirectedAi`. Because
`Order` actions only reach `DirectedAi` seats, the same beats are inert when a
person plays.

**Content:** chapters 1–2 of [story.md](../story.md) as director beats in
`operation-ferryman.json`:

- a milestone beat (no actions, audience blue) for every coach `beat` gate;
- red's `river_guns` cue: a `DirectedAi` order to fire on ground near the village,
  heard by blue;
- the chapter 2 forces worn as the story says.

Write the solution beats beside them.

**Seam:** `crates/sim/tests/scenario_walkthrough.rs` finds every scenario that has
a solution file and runs it to `finished()`, or to its last chapter's end while
the story is unfinished.

**Assertions (mechanics only, no content named):**

- every beat of every chapter fired;
- chapters opened in increasing order;
- no solution order was refused (`director_refusals()` is empty, and there is no
  per-unit refusal in the publication);
- the replay equals the live digest;
- finished within the solution file's own loose `max_minutes` tripwire. It is a
  tripwire, never a target to tune toward.

Until slice 17, the scenario's last chapter is chapter N without a
`MatchResult` end, and the human-side-wins assertion is enabled in 17. This is a
named short-lived relaxation, removed in 17.

**Playable:** the test writes
`throwaway/tutorial-walkthrough/<scenario>.replay.json`; open it with
`/battle?replay` and watch chapters 1–2 play themselves.

**Gates:** the test itself, run alone
(`cargo test -p sim --test sim scenario_walkthrough`). Record its wall time here
as the baseline for R3.

**Delegated:** the harness's step loop, and how the solution waits (always on
facts, never on ticks).

**Must stay green:** director tests, directed-seat tests, every digest.
