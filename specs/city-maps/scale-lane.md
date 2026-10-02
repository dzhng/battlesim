# Scale lane: a whole battle inside the tick budget

A second session works this lane in parallel with the map lane. It is simulation cost: the same battle, the same outcomes, fewer instructions. Nothing here needs art, and nothing here touches the generator, the menu or the renderer.

## The contract

A full battle on a full-size generated map sustains **at least 25 Hz measured
simulation throughput**, with 100 units a side, from the opening to late battle
(wrecks, felled trees, collapsed buildings, corpses). This is the user's
provisional acceptance floor, revised on 2026-10-02. Reaching 30 Hz and having no
tick over 33 ms are deferred optimization targets; neither blocks this lane's
closure on its own. Keep the nominal simulation timestep, game speed and rules
unchanged. Delivery, memory, replay and functional contracts still apply.

Where it stands ([sim lane closeout](assets/sim-lane-closeout/README.md), [SA5](slices/SA5-sight-cost.md), [S1](spikes/S1.md)): a quiet crossing of Metro Large has no tick over 33 ms, but the browser stress checks ran at 24.4 Hz early and 12.4 Hz late, and the dense 100-a-side endurance run has ticks over 33 ms (p95 27.8 ms, p99 47.0 ms).

## Ownership

| This lane owns | The map lane owns (stay out) |
|---|---|
| `crates/sim/src/` except `encounter/`, `maps.rs` and `world/surfaces.rs` | `crates/mapgen/`, `crates/contract/src/{ground,curve,maps,encounter}.rs`, `fixtures/maps/`, `fixtures/map-presets.json`, `fixtures/encounters.json` |
| `crates/sim/examples/` (the reports), `crates/sim/tests/` for what it changes | `apps/`, `packages/`, `web/src/`, `web/scenes/` |
| This file's Status section | `README.md`'s Next Agent Prompt and TODO |

A change this lane needs outside its column is a small, named commit, mentioned in Status. No repo-wide renames while both lanes are open: they cost the other lane a full re-merge.

## Work, in order

1. **Measure where the time goes.** `city_report` and `endurance_report` on generated maps (`mapgen generate-map`; usage in [S1](spikes/S1.md)), 100 a side, early and late, in instructions retired per system (sight, fog, navigation, movement, weapons and flight, publication). This is [C05 measuring tools](slices/C05-measuring-tools.md) as far as this lane needs it. Record the table here before changing anything.
2. **[C06 sim scale passes](slices/C06-sim-scale-passes.md):** take the largest costs first. Each pass leaves battle digests and replays unchanged, or is a named decision with the reason.
3. **[C07 publication at scale](slices/C07-publication-at-scale.md):** what the simulation hands the page each tick, including the late-battle growth (remains, known props).
4. **The browser stress checks** (`endurance` and `benchmark` scenes), with measured simulation throughput at least 25 Hz early and late on this machine. Retain actual rates and frame/tick distributions; run the full-map admission at the performance milestone, not before each commit or push.
5. **The whole pipeline over many seeds** (the tooling half of [C54](slices/C54-generation-gate.md)): one runner that, for each map type, size and a set of seeds, generates the map, plans the `assault` encounter and plays a short battle, and reports every refusal with its reason, units that never reach their goal, and tick cost. Fix what it finds in the simulation; report what it finds in the generator or the planner in Status.
6. **Two faults it will meet, known already:** infantry route times through a town come back infinite (`NavGrid::route_time`), so the encounter planner can only time a jeep ([C59 outcome](slices/C59-encounter-planner.md#outcome)); and a replay carries no engine build identity, so a build that changes simulation code without changing the scenario replays to a different battle unrefused ([C55 outcome](slices/C55-runtime-generation.md#outcome)).

## How to work

Read [`AGENTS.md`](../../AGENTS.md): narrow checks only, no full gate. A performance change is proved by unchanged digests (the digest and replay tests) and measured in instructions retired, never wall time; `village_report -- --quick --compare main` is the cross-check over whole battles. Branch from main and merge back at each green pass, having run the sim tests for what changed and the one or two scenes it can move. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Keep raw reports and logs in gitignored `throwaway/`; the spec gets conclusions and tables only.

**One GPU, shared.** Several sessions are working at once. Run every scene, render and asset sheet through the GPU lock in the main checkout (the README's Checks section has the command), never two at a time, and keep heavy jobs (a full Rust test run, a long sweep) to one at a time.

## Status

**C05 is complete; C06/C07 admission remains open.** Integrated fixes are on main
`17dd1704`. Fresh native/Wasm build `c1fb0f09…` matches, with physical catalogue
`6b0a5e8b…` and current rules `b7e67721…`. Focused navigation, planning/replay,
publication, authority/decoder and type checks pass. The complete historical
measurements freeze `13beacf9` / build `6b3eb26d…` / rules `a59680c1…`; they are
controls, not current admission.

Current pickup, in order:

1. Resolve current-main move-certification cost and workload validity before a
   long admission run. The unmodified Metro Large seed-4 probe spends
   **113,197.618 million instructions in Orders at its worst scripted tick**.
   At tick 150, only eight of blue's 74 first-wave fighters acquire goals and
   none of red's do. This is observed execution, not hidden command verdicts;
   distance alone does not explain the refusals. Keep destination truthfulness,
   physical rules, the validation allowance and the stress recipe intact while
   selecting a fix or explicitly separating that incoming movement work.
2. Run both complete 9,000-tick contact arms after that contract is resolved.
   The infantry refinement fix uses the existing failure/revision
   replan path when a newly known body closes a shared edge; its public regression
   proves a physically safe route, no pending job and exact serialized replay.
   Admit the lossless ground-tail correction on those fresh battles. The original
   early arm reached **33,496 B**, above the unchanged **19,800 B** maximum.
   Exact captured-record reconstruction now reaches **18,652 B** early; late is
   only a partial corpus because the old navigation panic interrupted it.
   Producer/decoder checks pass; no knowledge delay, quantization or allowance
   increase occurs. Snapshots, full active heap and real browser overlap remain
   measurement gates.
3. The three fresh short-sweep extensions now refuse all requested destinations.
   Their ten infantry remain idle through 120 s with null goals and no pending
   work. This does **not** resolve the historical planning delays: current
   certification prevents the routes from entering planning. Revisit that proof
   after movement admission is resolved; do not count refused orders as progress.
4. Run the full generated-map browser `endurance` milestone once native contracts
   pass. Admit at least 25 Hz early and late; retain actual distributions. The
   current `benchmark` loads the village: its short run is a regression control,
   not Large-city frame/tour admission. A short reset run proves resource lifetime
   only. No browser gate precedes each commit or push; parent full gates belong
   to parent closeout.

The historical matrix retains all 90 type × size × seed requests: 89 battles play
80,100 ticks and one encounter is refused, with no generation refusal or panic.
Mixed Medium seed 9 still fails the planner's approach-balance rule; it remains
a parent-lane finding, with no replacement seed. Ten infantry units remain
planning after 30 s. The three earlier frozen vehicle failures terminate in
120 s with explicit obstruction, living units and no pending work.

[C05](slices/C05-measuring-tools.md#frozen-pre-optimization-measurements) owns
initial measurements; [C06](slices/C06-sim-scale-passes.md) owns simulation and
reset evidence; [C07](slices/C07-publication-at-scale.md) owns delivery evidence.
The matrix implements the tooling half of [C54](slices/C54-generation-gate.md).
Infantry timing and replay build refusal are integrated; the parent's compiled
scenario replay-storage debt remains separate.

Named cross-boundary additions are the mapgen battle-sweep runner, native/Wasm
build identity, publication decoder/feed and generated endurance preparation/reset
harness. The configured Codex CLI review was unavailable for this account;
independent read-only agent reviews supplied the second opinion. No additional
optimization is selected unless a measured accepted contract fails.
