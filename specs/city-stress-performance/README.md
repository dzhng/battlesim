# CPU performance experiment

The candidate on `codex/city-stress-30pct` saves **30.04% of total simulation instructions** on the fixed 200-unit city workload, including its five-second warmup. All 9,150 per-tick battle digests and both sides’ packed publications match the unchanged baseline, as do final actors, firing and ammunition outputs. This is an experiment for the user’s complexity decision; it has not been merged to main.

## Measurement contract

The baseline is `ab896816e306f07b1ebd3f8cb988aee2a31980c8`. The primary measure covers all 305 simulated seconds, ticks 1–9150. Exact per-step retired instructions total **1,647,007,909,026** at baseline and **1,152,206,337,861** for the candidate. Savings are `1 − candidate / baseline`, against the requested 30% threshold. The margin above that threshold is small; this is a measured result for this control, not a guarantee for every map or battle.

Construction, publication packing, diagnostic digest calculation and rendering are excluded. The separate recorded 300-second window saves **33.35%** (1,366,197,581,381 versus 910,586,397,197). The initial evaluator excluded warmup; protocol 2 includes it and adds complete per-tick publication hashes. Results from those protocols stay separate. Retired instructions are the cost measure; wall time and FPS remain observations sensitive to machine load.

The native pair uses the preserved complete 10×10km Metro seed 4 physical map with SHA256 `ee6b5c6268ad6aeff32877c7e8e7fcc21c0b422c5c40a014edd14ce8d88fa070`, captured current rules, battle seed 4 and 100 actors per side. Its final digest is `308a322d55ad0e6d`. The physical control and raw research evidence remain in ignored scratch; reproducing this exact pair requires that saved map, rather than regenerating under different inputs.

The production-browser pair separately uses the current generated Metro seed 4 map with identity `187e4328403f0cbef49c1d896574c4d6944374988a92e815f17dd99303207825`. It has 836 forest shapes, versus 940 in the preserved native map. Each comparison stays within its own physical map. Native instruction savings cannot be presented as measured savings on the browser map. Browser controls retain the same request, scenario, 1920×1080 viewport, Metal adapter and camera tour.

## Effect and complexity

The initial independent trials share one baseline and the recorded 300-second window. Their gains do not add; the final combination is measured separately across the whole run.

| Strategy | Initial recorded-window saving | Decision and cost |
|---|---:|---|
| Native hardware math feature | 11.82% | Discarded: stable Wasm has no matching acceleration. |
| Guarded radius comparisons at five hot queries | 3.94% | Retained and extended to existing movement/clearance comparisons. |
| One rotation per footprint query | 3.17% | Retained: uses the existing rotation owner. |
| Union overlapping scenery buckets | 9.59% | Retained: transient intervals, no new persistent index. |
| Reuse unchanged observer visibility | 15.49% | Retained: bounded complete fields with conservative invalidation. |

Later whole-run trials established the tradeoff directly: route-leg caching brought the four-change incumbent to 28.27%; broader movement comparisons with that cache reached 29.91%. Removing the extra cache and extending fine-path comparisons yielded 29.53%. Evaluating route distance only when consumed reached 29.76%; sharing the existing hull snapshot’s bounds reached 29.88%; skipping an unused lane tail reached **30.04%**. All full trials preserve exact recorded outputs. An additional body-rotation cache was not promoted after its opening trial; removing the route cache confounded its standalone attribution, so it is not claimed as a measured isolated loss.

The [radius owner](../../crates/sim/src/math.rs) avoids portable square roots only when squared values are safely separated. A conservative rounding band and exceptional values retain the original pinned `hypot`. Four predicates preserve the original strict/inclusive comparisons. Actual movement and aim norms keep their original arithmetic.

The [scenery index](../../crates/sim/src/world/props.rs) merges per-row bucket intervals before reading entries. Actual and remembered scenery share the algorithm, with ascending unique candidates and the same loose query region. Transient work follows view-row intervals and union entries, avoiding repeated entries from overlapping eyes.

The [visibility owner](../../crates/sim/src/visibility.rs) keeps one complete field per observer. Exact eyes, sight, relevant sensor values, global body revision and cleared-ground count determine reuse. A miss runs the original sweep; each field joins the side’s fresh union, preserving the result when another observer moves or dies. Fields add about **39.1MB maximum capacity for 200 units** on this map. Dead entries remain bounded by the battle roster. Cold misses lose the across-observer early-union shortcut; frequent movement or geometry changes can reduce this benefit. Global invalidation avoids local revision machinery.

The [movement owner](../../crates/sim/src/movement/mod.rs) pairs each existing immutable hull snapshot with its radius once per tick, adding eight bytes per hull. The [soldier path](../../crates/sim/src/movement/soldier.rs) computes remaining route length only near the destination and constructs a lane tail only after its first segment passes. These remove unused work without new retained state. The low-value route-leg cache and body-rotation cache are absent.

Across production source, excluding test modules, the candidate adds 229 lines and removes 96: **133 net lines**, or 114 net nonblank lines after excluding full-line comments. The main maintenance cost is the visibility cache and its invalidation contract. There are no new dependencies, gameplay rules, reduced sensing frequency, coarser fog, changed admission budgets, postponed commands, routing architecture or 20km changes.

## Wave stalls

A wave stall is a long simulation-worker tick admitting many simultaneous scripted movement commands. Physical movement rehearsal owns nearly all that tick’s cost. The candidate reduces redundant calculations while retaining the same admission and scripts.

| Wave tick | Baseline instructions | Candidate instructions | Less work | Observed baseline / candidate pause |
|---:|---:|---:|---:|---:|
| 150 | 139,935,912,327 | 118,602,926,714 | 15.24% | 6.25 / 5.90 s |
| 3750 | 108,808,259,846 | 86,752,895,316 | 20.27% | 5.11 / 4.20 s |
| 7350 | 76,696,697,551 | 64,293,278,023 | 16.17% | 4.91 / 3.11 s |

Several-second pauses remain. These native wall times include scheduling and, in some candidate runs, concurrent correctness checks; instruction reductions are the stable evidence. A faster worker can improve simulation progress and reduce hitches without a proportional rendered-FPS increase.

## Browser confirmation

The matched five-minute production pair uses the same scenario fingerprint, physical-map identity, preparation request, camera tour/keyframes, viewport and Apple Metal adapter. Both sampled all 14 keyframes within the existing 100 ms gate. Its results are:

| Browser measurement | Baseline | Candidate |
|---|---:|---:|
| Average FPS | 25.74 | 27.00 |
| Simulated seconds in 300 wall seconds | 259.33 | 270.67 |
| Mean simulation step | 17.89 ms | 12.46 ms |
| Simulation step p95 | 35.13 ms | 18.04 ms |
| Worst simulation step | 12.08 s | 7.79 s |
| Mean GPU frame | 12.97 ms | 12.40 ms |
| Peak page heap | 1,680.7 MiB | 1,678.3 MiB |

Observed FPS improved **4.92%**, and the worst simulation pause shortened **35.52%**. Several-second wave stalls remain. The page heap is not a complete measure of the simulation worker’s retained visibility fields; its garbage collection also varies between runs. GPU texture bytes match, and buffer totals remain essentially unchanged.

Both runs miss the 30 FPS floor and the existing final-minute firing diagnostic. The candidate has eight own firing units in that late window. Neither threshold is changed, and the benchmark scene therefore exits with failure despite producing a complete report. Camera, rendering, sound, workload identity and contact checks otherwise pass. This single before/after pair is observational across different machine load; it does not establish a precise causal FPS percentage or broad performance guarantees.

## Verification and test audit

The complete check passed: format, lint, typecheck, all Rust tests including 627 simulation contracts, and all 129 web files / 960 tests. After final movement simplifications, the affected movement suite and fresh Wasm web suite pass again, with Clippy clean. The final native control compares every tick’s state and both packed publications; this covers fog output that the battle digest alone does not carry.

| Contract | Surviving proof and disposition |
|---|---|
| Numerical comparisons, ties and exceptional values | Keep the math oracle against the original portable norm; strict and inclusive cases share one test. |
| Query union and edits | Keep actual candidate-value checks through overlapping/disjoint/clamped views and body movement/removal. |
| Visibility invalidation and independent observer fields | Keep fresh-sweep output comparisons for eye/sight/sensor/body changes, clearing and observer disappearance. Clearing and ghost-union regressions were deliberately falsified and restored. |
| Movement and replay | Keep physical gap, soldier-disc, endpoint/slide, replay and admission suites, plus exact full-run outputs. |
| Cost measurement | Keep the scratch research harness; omit process-wide counter assertions from parallel unit tests because unrelated test threads contaminate them. |
| Live tuning versus test controls | Repair three baseline failures: pin recon optics in forest/pursuit tests and associate exterior garrison eyes with their living indoor bodies. Gameplay fixtures stay unchanged. |
| Native/Wasm paired records | Repair stale generation/publication outputs from unchanged baseline runtime; keep identical requests and all 1,700 contact side/resync inputs. Candidate and Wasm match them. |

Eight generation records and four simulation tests failed on unchanged baseline as well. Repairs preserve their behavioral claims. Main’s recon tuning changed generation proof inputs; one previously refused seed now succeeds, so its status-specific label is removed. Contact lifecycle refresh changes fog/publication hashes while retaining authoritative digests. These repairs are separate from performance implementation.

The final review covered shape, code and docs. Independent read-only review found no remaining correctness defect. It identified unnecessary all-route work in the discarded route cache; that cache is removed. The configured local CLI model was unsupported, so an available Codex subagent supplied the second opinion without changing model settings.

Raw evidence, the fixed contract, attempt ledger, candidate snapshots and comparison live under `throwaway/city-stress-30pct/`. The [endurance report](../../crates/sim/examples/endurance_report.rs) and [browser benchmark](../../web/src/battle/benchmark/README.md) remain the product’s benchmark owners.

## Next Agent Prompt

The experiment is complete and remains separate from main. Review the saving against the added visibility-cache contract and approximately 39.1MB field capacity, plus the small shared-query and arithmetic changes. The performance and test-record repair commits are separate. Do not merge to main without the user’s decision. Preserve the complete city-maps generation report and the saved scratch evidence; further renderer FPS work or admission scheduling would be a separate experiment.
