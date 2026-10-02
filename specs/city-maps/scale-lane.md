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

## How to work

Read [`AGENTS.md`](../../AGENTS.md): narrow checks only, no full gate. A performance change is proved by unchanged digests (the digest and replay tests) and measured in instructions retired, never wall time; `village_report -- --quick --compare main` is the cross-check over whole battles. Branch from main and merge back at each green pass, having run the sim tests for what changed and the one or two scenes it can move. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Keep raw reports and logs in gitignored `throwaway/`; the spec gets conclusions and tables only.

## Status

**C05 native measurement pass complete; C06 next.** Branch `codex/city-maps-scale` starts at
`732f3db0` (the supplied checkout), which contains the supplied map/simulation integration. Local `main` advanced
while measuring; its incoming changes are evidence/docs pruning and removal of
obsolete serde defaults, so merge those at the green checkpoint. Keep the exact
measured inputs for same-rules parity.

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

1. C06: movement is largest in actual contact. Soldier fine-route work still
   uses full surface classification for boolean walkability; inspect its cost
   and the blocked rejoin search's potential no-progress loop before fog caches.
2. Retain all matched city/saved digests and replay. Run only affected tests,
   then compare reports with exactly the same map and stress inputs.
3. C07: bound delivery growth without changing complete logical observations.
   Existing fog/ground deltas are bounded; bodies/corpses remain complete each
   tick. The provisional 19.8 KB/tick target misses 100-unit own rows alone; any
   budget change must be an explicit workload decision.
4. Build stable Wasm, then run `endurance` and `benchmark` scenes serially.

Map-lane coordination: no generator, fixture, browser, renderer or parent README
changes. C07 producer/decoder changes, if measurements select them, will be a named
cross-ownership commit. The configured Codex CLI review could not run because its
`gpt-6.1-sol` model is unsupported on this ChatGPT account; independent read-only
subagent review found and corrected generated-workload, attribution and unavailable-
counter reporting issues. Focused authority/replay, city stress/placement and late
aggregate-ID regressions pass; focused clippy passes. The late stress fix allocates
wreck IDs after all authored physical parts, preserving existing saved-map IDs.
