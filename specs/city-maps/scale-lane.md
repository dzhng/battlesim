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

**Acceptance components pass; final review and scoped archive are next.**
All simulation and delivery passes are on main. The reset harness correction is the final
named cross-boundary pass; it preserves the existing reset-allocation report and
adds exact presentation-state evidence. No further optimization is selected.

- **Browser floor passes on the identified full reference.** Frozen layout 9 /
  presets 8, Metro Large seed 4, 100 living units a side, engine `d58fb441…`,
  production source `9211c627` on presentation lineage `d805a1ea`. The complete
  five-minute arms measure **28.377 Hz early / 26.067 Hz late**, with 61 / 68 own
  units firing. The short village benchmark passes. The late page-plus-worker
  snapshot is 1,865,250,762 B; sampled dedicated process-tree RSS peaks at
  6,212,878,336 B, with shared-page and sampling caveats. These are separate
  measures, not a requested-heap ceiling.
- **Exact reset admission passes.** Three complete generated controls have the
  same tick, settled clock, digest, 49 corpse IDs, camera, buffers, textures and
  bytes. The actual retained 16-byte-buffer mutant fails the exact check.
  The tracked saved-field scene also passes; the tracked full generated mutant
  fails only resource equality while presentation remains exact. The original
  five-minute fixture's red reset comparison remains recorded as historical
  evidence, not relabelled green.
- **Full functional and delivery admission remains identified.** Source
  `82372fe7`, engine `1d9d85aa…`, rules `b00f7d5e…`, the same frozen reference:
  both 9,000-tick arms match all 9,001 Native/Wasm digests and raw publications.
  Maximum complete active records are 17,152 B early / 19,328 B late, under the
  unchanged **19,800 B** limit. Complete production decoding, raw ground values,
  retained snapshots, malformed retry, resync and side switching pass.
  The subsequent pure hull-filter ordering returns identical hull geometry and
  order, with unchanged allocations and state. Its separate 901-state and
  complete two-side-observation pairs justify narrow reuse; no new 9,000-tick
  run or heap measurement is claimed. Matched late whole-Weapons work falls
  35.418%; construction plus stepping falls 7.813%, with early nonregression.
- **Requested-memory admission passes on that full proof.** Active requested
  peaks are 418,616,216 B early / 457,699,995 B late, below the **4 GiB** guard;
  conservative late reallocation overlap is 458,088,603 B. Actual Wasm linear
  capacities reach 489,029,632 / 499,974,144 B. These component scopes exclude
  additional browser/page/worker JavaScript heaps and GPU allocations. Portable numerical corrections and
  shorter anchors remain named trade-offs, not CPU gains.

Current pickup: commit the reviewed reset correction, consolidate the 23 scale
choices and close only this lane. Preserve incoming parent work and historical
control identities. The parent generator/presentation lane remains active.

The historical matrix retains all 90 type × size × seed requests: 89 battles play
80,100 ticks and one encounter is refused, with no generation refusal or panic.
Mixed Medium seed 9 fails the planner's approach-balance rule, with no replacement
seed. Ten historical infantry units remain planning after 30 s. The earlier frozen
vehicle failures terminate in 120 s with explicit obstruction. The repaired
public infantry timing, town-corner and living-member-start regressions pass with
actual arrival and replay equality. Fresh historical extensions refused before
planning do not resolve the separate generated-journey finding; that remains a
parent follow-up. The parent's compiled-scenario replay storage debt is separate.

[C05](slices/C05-measuring-tools.md#frozen-pre-optimization-measurements) owns
initial measurements; [C06](slices/C06-sim-scale-passes.md) owns simulation and
reset history; [C07](slices/C07-publication-at-scale.md) owns delivery history.
The matrix implements the tooling half of [C54](slices/C54-generation-gate.md).
New parent layout/presentation inputs do not relabel the frozen reference.
