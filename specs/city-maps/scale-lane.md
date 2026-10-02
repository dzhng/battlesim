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
6. **Historical faults now repaired:** infantry route timing uses the refined
   connectors and living-member starts described in C06; replay admission carries
   and checks the engine build identity. Their focused public proofs remain part
   of the lane's regression coverage. The parent's compiled-scenario replay
   storage debt is separate; it does not reopen these completed repairs.

## How to work

Read [`AGENTS.md`](../../AGENTS.md): narrow checks only, no full gate. A performance change is proved by unchanged digests (the digest and replay tests) and measured in instructions retired, never wall time; `village_report -- --quick --compare main` is the cross-check over whole battles. Branch from main and merge back at each green pass, having run the sim tests for what changed and the one or two scenes it can move. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Keep raw reports and logs in gitignored `throwaway/`; the spec gets conclusions and tables only.

**One GPU, shared.** Several sessions are working at once. Run every scene, render and asset sheet through the GPU lock in the main checkout (the README's Checks section has the command), never two at a time, and keep heavy jobs (a full Rust test run, a long sweep) to one at a time.

## Status

**Implementation checkpoints are on main; final admission is running.**
The `b5c63565` control of move certification leaves the former stress opening mostly inactive: three
rounds in 900 ticks. The named workload correction starts the same 200 living
units 200 m closer to contact, using the existing placement projection. It keeps
the full Metro Large seed-4 world, all scripts, unit types, rules and late remains.
The unmodified core on that corrected input launches 8,835 early / 19,579 late
rounds in 900 ticks. This validates combat cost; it does not repair refused moves.

Two exact movement cost passes are integrated: avoid local steering geometry
when no member can steer, and omit cover-position rows neither immutable side
knowledge can read. Their paired whole-Orders gains are 25.7% early and 13.69%
late respectively, with identical every-tick digests and complete observations.
Focused movement, cover, sensing, delivery and city-placement checks pass.
The browser harness requires rising shot counters from at least ten own units
during measurement in each generated arm lasting 30 seconds or more.

Current pickup: diagnose late throughput and reset-state equivalence, then rerun
the affected browser gate before closing the lane.

1. **Current Native/Wasm admission passes.** Frozen source `82372fe7`, engine
   `1d9d85aa…`, rules `b00f7d5e…`, layout 9 / presets 8, Metro Large seed 4.
   The base generated/early map is `bdadf296…`; late runtime map `553e6e05…`
   includes the synthetic wrecks. Both 9,000-tick arms match all 9,001 authoritative
   digests and raw publications. Maximum complete active records are 17,152 B
   early and 19,328 B late, below the unchanged **19,800 B** limit. Complete
   production decoding, raw ground values, retained snapshots, malformed-record
   retry, resync and side switching pass. Portable evaluation is a named native
   last-bit correction, not a CPU gain. Shorter exact anchors retain the wire
   grammar at a measured encoding/index-memory cost.
2. **Native component memory proofs pass.** Separate requested-layout diagnostics
   have active peaks of 418,616,216 B early and 457,699,995 B late, with conservative late
   reallocation overlap 458,088,603 B. Both are below the **4 GiB** guard and
   finish with the System-counter run's digest. Actual Wasm linear capacities
   reach 489,029,632 B and 499,974,144 B respectively. These component measures
   exclude page, worker and GPU allocations. Whole-browser memory and reset
   resources remain part of the pending browser milestone. Snapshot sizes and
   decoder/copy overlap are reported separately; the codec's 64 MiB ceiling is
   not a snapshot performance budget.
3. **Browser admission is red on merged presentation source `d805a1ea`.**
   The complete five-minute arms reach **27.120 Hz early / 22.004 Hz late**;
   late misses the unchanged **25 Hz** floor. Actual rising shot counters cover
   61 / 68 own units. The short village benchmark passes at 44.4 FPS; it does
   not prove Large-city throughput. Late page-plus-worker measurement is
   1,873,889,998 B; sampled dedicated process-tree RSS peaks at 5,888,409,600 B
   (may double-count shared pages, not a requested-heap ceiling).
   Three paused tick-90 resets retain 591 buffers / 41 textures, but buffer bytes
   differ by up to 832 B. Static corpse buffers are a concrete 64 B-per-body
   candidate: equal final tick need not imply equal presentation death history.
   Attribute allocation labels before changing the comparison; retain the exact
   leak check. A scratch browser probe measures actual worker step time and page
   CPU cost, then those reset states. Pure hull eligibility ordering is the next
   Native cost candidate, not a claimed gain. Keep the floor, rules, timestep,
   complete delivery and memory scopes. Parent full gates remain parent-owned.
4. Complete the whole-owned review and consolidate the scale choices ledger.
   The original timing, town-corner and living-member-start regressions pass
   through current move certification with actual arrival and every-tick replay
   assertions. The three fresh 120 s historical infantry extensions are refused
   before planning; those idle results do not resolve the separate generated
   journey finding. Preserve that parent follow-up when archiving this lane.

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
