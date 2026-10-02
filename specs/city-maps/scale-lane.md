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

**C05 is complete; C06/C07 admission remains open.** The current measurements
freeze simulation source `13beacf9`, native/Wasm build `6b3eb26d…`, layout-7,
physical catalogue `6b0a5e8b…` and rules `a59680c1…`. Native/Wasm identities
match, and focused navigation, planning/replay, publication, authority/decoder
and type checks pass. Historical measurements are controls, not current admission.

Current pickup, in order:

1. Fix the late contact run's infantry connector panic in `navigation/foot.rs`.
   Preserve the exact failing input and pin the behavior with a narrow regression.
2. Close the whole-record delivery miss. The completed 9,000-tick early arm
   reaches **33,496 B**, above the unchanged **19,800 B** maximum; newly learned
   ground runs contribute **21,008 B**. Late stops on the navigation panic.
   Test a lossless ground representation against the captured records before
   changing production. No knowledge delay, quantization or allowance increase.
3. Diagnose the ten infantry units still planning at the short sweep boundary.
   Then rerun only the affected cases and final early/late native admission,
   including snapshots, browser copying/decoding and peak memory overlap.
4. Run the full-map browser `endurance` and `benchmark` milestone once the native
   contracts pass. Admit at least 25 Hz early and late; retain actual distributions.
   A short reset run proves resource lifetime only. No browser gate precedes
   each commit or push, and parent full gates belong to parent closeout.

The current matrix retains all 90 type × size × seed requests: 89 battles play
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
