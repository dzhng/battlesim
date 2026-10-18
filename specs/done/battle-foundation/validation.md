# Verification and review

This document defines the evidence each slice must produce. The gates exist and run (commands in the README's standing facts); measured results live in each slice's verdict.

## Behavioral gates

Use pure native Rust tests at the named seams, TypeScript tests for the external boundary/UI, and browser scenes for actual WASM+GPU integration. Tests pin user-observable behavior, transitions, and physical invariants; do not assert arbitrary tuning constants or clone implementation formulas. Independently analytic collision examples and carefully fixed state transitions are stronger than long lucky simulations. Probabilistic outcomes use paired seeds and distributions; failures report state/event traces rather than merely a checksum.

The single worker and direct simulation must agree tick for tick in the same WASM build; same-build replay digests must match. Cross-native/WASM floating-point bit identity is not promised. On mismatches report the first differing tick and subsystem. Never loosen tolerances without showing why the comparison was invalid. Run formatting/lint/typecheck and appropriate prior behavioral gates before a slice is accepted; avoid unrelated broad reruns when nothing changes their inputs.

The first setup slice installs the project-native task runner: root `bun run build`, `bun run check`, `bun run verify`; native `cargo test --workspace`; web `bun run --cwd web test`, `typecheck`, `lint`; registered scenes `bun run --cwd web scene -- <fixture-id>`. Establish and verify the exact scene argument parser in that slice.

## Information boundary tests

Give two worlds the same public map, own state, and observed events. Change only unobserved enemy health, identity, motion, orders, ammo or hidden dynamic prop updates. Until a lawful new event occurs, the side observation, target choice, reason messages, sound cues, pick results, light/effect outputs and bot commands must remain equal. Changing camera position may alter rendering, never observation or hearing eligibility. Debug inspection must be unreachable from the player route and controller imports.

A shared exact contact can aim an ordinary tank but cannot sustain the launcher's supported missile. A 1.5-second acquisition grace remembers the last observation rather than querying the true target. Contacts remain stable between evidence events; unlimited speculative fire is an intentional user choice, not an information-leak exception.

## Visual evidence

Per-slice files specify one visual variable and a crop. Capture 1280×800 DPR 1 as the primary framing and 900×600 for constrained HUD layouts; record real browser/channel, adapter, seed, tick, camera and config digest. The registered scene regenerates full frames and 2×–4× feature crops into gitignored `throwaway/evidence/<fixture-id>/`; copy a prior capture aside before rerunning when a comparison needs it. Commit conclusions (metrics, critique findings and dispositions) in the slice verdict, not the images. Failure captures stay in the evidence set for the run.

Use compare-screenshots for prior/reference comparisons, with a target-based less-wrong verdict. A baseline can be wrong; identical pixels are not correctness. Run a fresh screenshot-critique agent as the **last visual acceptance check**, supplying only candidate shots, crops and a neutral prompt. Inspect high-confidence findings and record their disposition. A passing image does not prove hidden-state, collision, or performance correctness.

Human review is non-blocking for reversible implementation decisions. Use preview-shots for one coherent Preview set and allow about five minutes while doing independent work; never block a tool wait beyond 60 seconds. With no response, decide from evidence, record why, close the opened set, and proceed. User-rule changes still require a decision, not assumed consent from silence.

## Performance targets and workload

Planning host: Apple M5 Pro, 48 GiB RAM, arm64. Re-probe the implementation host/browser; current-machine-first is not an adapter guarantee. Initial **proposed targets**, measured at 1280×800 DPR 1 on hardware WebGPU: real-time simulation sustains 30 Hz with p95 step <20 ms and p99 <30 ms; village rendering p95 frame interval <25 ms and p99 <50 ms with average near 60 FPS. Steady active simulation+known GPU allocation budget is 1.5 GiB; record process/browser memory separately because counters are not interchangeable. The first slice records a baseline; slice 07 identifies projectile cost; subsequent slices show meaningful deltas.

These targets are not claimed measurements and may be revised with explicit evidence and explanation before implementation gates are frozen. Do not silently reduce battle size, skip physical rounds, cap speculative exchanges, or delete persistent remains to satisfy a budget.

Slice 16 uses 100 units/side, including 100 total eight-person rifle squads plus 100 mixed vehicles/specialists. Add varied sensors, simultaneous 6k–8k rounds/second stress bursts, route changes, supply replacements and casualty turnover. Configure finite weapon range/lifetime so active flight is bounded by firing and travel, not battle duration. Retain corpses/wrecks through a 60-simulated-minute run; include a synthetic late state with 20,000 corpses and 2,000 wrecks (stress input, not expected ordinary play). Store inactive remains compactly; draw by visibility/LOD and query through spatial indexes.

Report sim tick p50/p95/p99, frame intervals, command-to-applied tick latency, publication bytes/credits/age, active projectiles, observation query count, collision candidates, path search count and expanded nodes, live resources, remains count and memory. Add a real-time rendered late-state run of at least five minutes alongside accelerated simulation soak; do not conflate simulated minutes with wall-clock rendering endurance. Withheld consumer credit, hidden tab, shader failure, worker failure and reset loops must recover/stop explicitly without busy loops.

## Completion claims

Slice 15 demonstrates the first playable tactical loop; slice 16 is the required scale-risk verdict for handing that checkpoint over. Full-match mechanics are not complete until continuation slices 17–22 ship and their evidence passes. No compatibility/migration work, networking, final art, campaign or deck-building completion is implied by either milestone.
