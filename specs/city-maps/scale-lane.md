# Scale lane: a whole battle inside the tick budget

A second session works this lane in parallel with the map lane. It is simulation cost: the same battle, the same outcomes, fewer instructions. Nothing here needs art, and nothing here touches the generator, the menu or the renderer.

## The contract

A full battle on a full-size generated map holds the simulation's budget: **no tick over 33 ms, and the browser's stress checks at 30 Hz or better**, with 100 units a side, from the opening to late battle (wrecks, felled trees, collapsed buildings, corpses).

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
4. **The browser stress checks** (`endurance` and `benchmark` scenes) at 30 Hz, early and late, on this machine.
5. **The whole pipeline over many seeds** (the tooling half of [C54](slices/C54-generation-gate.md)): one runner that, for each map type, size and a set of seeds, generates the map, plans the `assault` encounter and plays a short battle, and reports every refusal with its reason, units that never reach their goal, and tick cost. Fix what it finds in the simulation; report what it finds in the generator or the planner in Status.
6. **Two faults it will meet, known already:** infantry route times through a town come back infinite (`NavGrid::route_time`), so the encounter planner can only time a jeep ([C59 outcome](slices/C59-encounter-planner.md#outcome)); and a replay carries no engine build identity, so a build that changes simulation code without changing the scenario replays to a different battle unrefused ([C55 outcome](slices/C55-runtime-generation.md#outcome)).

## How to work

Read [`AGENTS.md`](../../AGENTS.md): narrow checks only, no full gate. A performance change is proved by unchanged digests (the digest and replay tests) and measured in instructions retired, never wall time; `village_report -- --quick --compare main` is the cross-check over whole battles. Branch from main and merge back at each green pass, having run the sim tests for what changed and the one or two scenes it can move. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Keep raw reports and logs in gitignored `throwaway/`; the spec gets conclusions and tables only.

**One GPU, shared.** Several sessions are working at once. Run every scene, render and asset sheet through the GPU lock in the main checkout (the README's Checks section has the command), never two at a time, and keep heavy jobs (a full Rust test run, a long sweep) to one at a time.

## Status

**C05 is complete; C06/C07 timing and delivery admission remain open.** The
integrated representation and fog-cache checkpoints are on `main` (`4efec25b`);
this branch also includes main through `9810912b`. Historical tables below use
original rules, map and catalogue. Current projectile tuning and `layout-6` need
fresh measurements; new results are not performance-only parity against those
historical battles.

Current pickup: finish the running complete pipeline matrix and continue the
measured fog/publication costs. The pipeline
tool is on main (`07a6f792`). Replay build identity is integrated and checked.
The route-start correction and variable-span publication checkpoint are integrated;
merged publication tests (13), the body-clear squad arrival/replay regression (1),
and browser authority/observation/codec tests (39) pass. Native and Wasm share the
new engine identity (`0f4153f7…`). The saved endurance scene reached startup but failed on a missing
effect flipbook's LFS content; the required runtime/effect assets are now fetched.
No browser throughput verdict is claimed.
No generator, menu, renderer or parent handoff implementation changed here.

The native reports now attribute the production tick without introducing OS
counters into the ordinary tick. Observation construction and wire packing are
separate costs. The generated contact load (`city-arena-1`) concentrates the
existing 100-a-side stress recipe and synthetic remains in a central 3 × 2 km
arena while loading the entire physical world; edge-to-edge movement remains
`city_report`'s separate load. Living starts use production placement queries.
This is synthetic local combat at city density, not an encounter playability proof.

### Before optimization

Apple arm64, release; same game rules/seed 1. Counts include native profiling
counter-read overhead. Raw records are in `throwaway/scale-lane/`.
Generated input: Metro Large seed 4, `layout-5` / `layout-presets-6`, prototype
catalogue `ed9981b358116490fd50585874a615ac427af819628e097cc2bf4216cda4f2cb`;
10 × 10 km, 12,889 buildings. No rendered surroundings are included in native cost.

| System | Saved endurance early, mean M/tick | Saved endurance late, mean M/tick | Metro Large crossing, mean M/tick |
|---|---:|---:|---:|
| Orders | 0.007 | 0.009 | 0.070 |
| Navigation | 0.504 | 1.741 | 14.573 |
| Movement | 31.286 | 52.903 | 12.754 |
| Flight | 0.422 | 3.209 | 0.952 |
| Sight | 8.094 | 8.487 | 6.343 |
| Fog including learning | 116.010 | 119.670 | 89.958 |
| Weapons | 1.097 | 4.989 | 0.762 |
| Observation construction | 5.025 | 21.174 | 1.691 |
| Other | 1.868 | 3.058 | 0.357 |
| Wire packing | 0.833 | 1.389 | pending |

Saved endurance rows cover 60 simulated seconds, before opposing forces engage;
late starts with 20,000 corpses and 2,000 wrecks. Digests:
`6d20f8bc729cb239` early; `f422e23d71026520` late.
Generated crossing covers 30 s, 100 a side, 50 shelled trees and added wrecks;
digest `bce285bd5ce8c08e`. It retires 128.9 G instructions; tick p95 36.44 ms,
p99 40.23 ms, max 43 ms, **123/900 ticks over 33 ms**. These native timings
include profiling and do not replace the browser admission gate.

### Generated contact baseline (before optimization)

The final `city-arena-1` front positions put the forces within weapon reach;
the earlier rear-edge exploration fired zero rounds and is excluded as contact
proof. Both rows below cover 60 s on the exact Metro Large input above. Synthetic
late starts retain 20,000 corpses and 2,000 wrecks. Profiled step costs exclude packing.

| System | Early mean M/tick | Late mean M/tick |
|---|---:|---:|
| Orders | 0.012 | 0.012 |
| Navigation | 6.365 | 7.742 |
| Movement | 80.371 | 115.302 |
| Flight | 1.090 | 3.923 |
| Sight | 28.688 | 29.728 |
| Fog traversal/candidate gathering | 51.323 | 55.884 |
| Ground/body/corpse learning | 13.391 | 16.703 |
| Weapons | 13.856 | 19.222 |
| Observation construction | 8.486 | 25.395 |
| Other | 1.707 | 2.839 |
| Wire packing | 1.306 | 1.822 |

Early: 369.5 G step instructions, p95 61.1 ms, p99 81.1 ms, max 131 ms,
610/1800 ticks over 33 ms, 149 rounds, digest `32ba0e3c667a728b`.
Late: 498.2 G, p95 56.6 ms, p99 78.8 ms, max 102 ms,
623/1800 ticks over 33 ms, 124 rounds, digest `1afb9cf97c2120f3`.
Movement maxima are 780.636/1014.755 M instructions, larger than the fog peaks.

Steady delivery averages 68,232 B early and 591,572 B late. Late corpses alone
average 522,161 B; fog averages only 565/560 B with existing exact deltas. Opening
fog snapshots are 390,632 B. Native packing is cheap relative to stepping, but
full browser copying/decoding remains unmeasured. Snapshot/resubscription budgets
remain G0's open decision; the 64 MiB codec ceiling is not a performance target.

### Priority and next pickup

1. C54 tooling: `crates/mapgen/examples/battle_sweep.rs` is the committed, named
   cross-ownership addition. It uses existing generation/compiler and assault
   planner APIs, current game admission, and the real Battle. A first Open Small
   seed-1 30 s sample advances every column unit; distant goals remain pending.
   The complete nine-cell, ten-seed matrix is now running against the current
   catalogue; every request retains its seed and outcome.
2. C59: legal local infantry links and their route-start correction are green. A public
   Battle regression exposes a physically clear member that cannot stand in the
   sampled navigation grid being selected as route start; the old search arrives
   on the same input. The shared start owner now prefers living members admitted
   by the side-known grid; the exact case arrives with physical clearance and
   same-build replay equality. C55 replay
   build refusal is verified; compiled scenario storage
   remains the parent lane's separate debt.
3. C06/C07: local fog invalidation and exact fixed/variable span copies are integrated.
   Measure early/late active Large bytes, p95/max and instructions afresh before
   selecting the next owner. Fog candidate/kernel attribution and immutable static
   decoder-view reuse proceed independently. Historical mean reductions do not close admission.
4. Browser: run `endurance` and `benchmark` serially, then establish the full
   generated-map stress arm. The existing saved stress arena alone does not prove
   full-extent loading. Snapshot/copy/decoder overlap remains a separate gate.

Map-lane coordination: no generator, renderer or parent README changes. The
C07 producer/decoder checkpoint is the named cross-ownership commit: it updates
wire fixtures and browser decoding while preserving canonical observation bits. The configured Codex CLI review could not run because its
`gpt-6.1-sol` model is unsupported on this ChatGPT account; independent read-only
subagent review found and corrected generated-workload, attribution and unavailable-
counter reporting issues. Focused authority/replay, city stress/placement and late
aggregate-ID regressions pass; focused clippy passes. The late stress fix allocates
wreck IDs after all authored physical parts, preserving existing saved-map IDs.

### C06 termination checkpoint

The blocked-end search fails safely on the previous infinite sampler and passes
with a bound derived from the remaining corridor. First-open and endpoint
selection remain unchanged. Movement-file tests (13), two rejoin tests, crossing/
slope/bounds query parity, authority/replay and focused clippy pass.

Matched final contact digests remain `32ba0e3c667a728b` / `1afb9cf97c2120f3`.
The first pass retires 369.9 / 499.4 G step instructions versus 369.5 / 498.2 G
before; movement averages 80.343 / 115.399 M instructions per tick, effectively
unchanged. Loaded-clock results are not throughput acceptance; **budget remains
red**. C07 proceeds in its own worktree against the completed C05 seam.

### C06 live traffic and sight bounds checkpoint

Traffic scans living, standing squads in the same order, instead of scanning
all fallen squads for every vehicle. Footprint radii are derived once per
immutable sensing call and once per movement gather, then refreshed after
squad movement or hull shoves. The authoritative geometry and collision order
stay unchanged. The process-counter regression runs in an isolated child so
concurrent tests cannot contaminate its measurement.

Matched 60 s contact runs retire 356.5 / 457.5 G step instructions, against
369.5 / 498.4 G before this pass. Movement drops to 77.863 / 96.912 M/tick;
sight to 23.941 / 25.088 M/tick. Both exact battle digests remain unchanged.
Loaded clocks still miss the tick budget; these are instruction gains, not
throughput acceptance. The finer diagnostic brackets are removed after locating
cover planning, so the production report keeps its existing system contract.

### C06 impossible engagement checkpoint

A fight candidate starts inside the squad's holding area and can lean at most
the rule's maximum distance beyond it. When every aim lies beyond weapon range
plus those two bounds, its engagement search has no possible result. The exact
range check retains edge leans and a micrometre of numerical slack. It changes
no resolution timing, chosen place, sight or fire rule.

The public battle cost regression fails on the old search: a distant visible
enemy raises peak movement cost from 405,912 to 85,232,244 instructions. It passes
with the range rejection. Dropping the lean allowance fails the positive boundary
regression. All 18 cover tests and 12 authority/replay tests pass, with focused
clippy clean. Shared cost-test isolation prevents cross-test counter contamination.

| Cost on matched 60 s city contact | Before this pass | After |
|---|---:|---:|
| Early step, G instructions | 356.5 | 278.9 |
| Late step, G instructions | 457.5 | 348.7 |
| Early movement, mean M/tick | 77.863 | 34.203 |
| Late movement, mean M/tick | 96.912 | 36.557 |

Digests remain `32ba0e3c667a728b` / `1afb9cf97c2120f3`; rounds remain 149 / 124.
Loaded clocks are not timing admission. The native and browser budgets remain open.

### Current combined Large contact measurement

Fresh `layout-6` Metro Large seed 4 under game admission: 10 × 10 km,
12,872 buildings, 22,503 authored parts, 231,180 bay positions and 46,710 ground
points. Physical map hash `f9e581249eb8e3467c4bc8fa736db952e384a4cf0c6fc382d2267451337d8bf5`;
prototype catalogue unchanged. Rules are main's `6de4cbf7` projectile tuning.
Each synthetic contact arm runs 60 s, with 100 living units per side. Late adds
20,000 corpses and 2,000 wrecks. These are fresh costs, not parity comparisons
with the historical `layout-5`/old-rule rows.

| System | Early mean M instructions/tick | Late mean M/tick |
|---|---:|---:|
| Navigation | 9.852 | 9.784 |
| Movement | 33.360 | 36.389 |
| Sight | 24.153 | 25.429 |
| Fog | 52.055 | 56.669 |
| Learning | 13.374 | 16.765 |
| Weapons | 13.548 | 19.408 |
| Observation | 8.454 | 25.558 |
| Wire packing | 1.613 | 4.910 |

Early retires 283.6 G step instructions, launches 180 rounds and ends at digest
`44aeaeff205a6b1a`. Late retires 354.2 G, launches 120 and ends at
`e4f0273adade82cf`. The profiled process-CPU maxima are 45.29 / 88.54 ms;
loaded wall maxima are 479 / 896 ms. **The 33 ms budget is still red.** Fog is the
largest recurring owner; movement peaks at 46.821 / 49.695 M instructions.

| Encoded delivery | Early | Late |
|---|---:|---:|
| Approximate mean B/tick | 17,144 | 16,781 |
| p95 B/tick | 48,600 | 48,540 |
| Max B/tick | 66,368 | 66,668 |
| Corpse mean / max B | 12 / 12 | 52 / 1,268 |
| Other rows mean / max B | 15,760 / 56,392 | 15,360 / 55,312 |
| Cold initial blue snapshot B | 452,836 | 952,120 |
| Red resubscription at 60 s B | 1,195,300 | 1,711,004 |

The sparse corpse copies remove the retained-tail amplification from this arm,
but other rows still miss the provisional 19.8 KB/tick limit. Means do not close
C07. Complete snapshots include exact fog and ground state; their packing at
60 s reaches 71–76 M instructions. Browser copy/decoder cost and peak overlap
still need their own proof. Raw current reports remain in `throwaway/scale-lane/`.

### Replay engine identity checkpoint

Replay now requires an automatic source/compiler/dependency/cfg fingerprint.
A foreign build is rejected before scenario checks or Battle construction, and
a missing field is refused; there is no historical replay mode. The fingerprint
excludes art and platform/profile differences within the supported deterministic
native/Wasm contract. It adds one field/comparison, with no tick or digest change.
The sim build script reuses the existing contract hash owner; its build dependency
is the named Cargo/Wasm cross-ownership seam.

The merged native authority/build-scope tests pass (15), and worker/observation/
codec tests pass (37) on root's rebuilt Wasm. Fresh native/Wasm build outputs share
the same identity (`639a5825…`); whole pipeline, browser throughput and compiled-
scenario replay storage remain separate outstanding contracts.
