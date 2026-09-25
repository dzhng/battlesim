# 16 — Current-host scale and late-battle verdict

**Status:** complete 2026-09-25. **Dependencies:** 15. **Milestone:** Village checkpoint.

## Contract and question

Can the architecture sustain full target populations and persistent battle history?

User requirements owned or exercised: S04, W11, M06. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`apps/battle-lab` load scenarios and harness telemetry consume actual simulation/resource counters; no second benchmark sim.

## Runnable artifact

/lab/endurance: 100 units per side, then dense speculative fire, seeded reinforcement/casualty turnover and accumulated remains. Report actual browser/adapter/config. Village stays the play checkpoint; stress is synthetic, clearly labeled.



## Verification and verdict

60 simulated minutes with real projectile/knowledge/supply operations and remains retention; repeated-session cleanup; no wall-clock catchup spiral; bounded publication credits and path work; compare frame/tick/memory to validation.md targets. Distinguish accelerated sim soak from real-time render run. Never cap bullets or erase remains to pass.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Late-state readability under load**. Review crop/mask: **Same full camera and dense contact/remains crop at early/late states**. Explicitly out of scope: New art and lowering rules to meet budgets.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Indexing/batching/culling optimizations preserving outcomes; measured budget revisions require documented rationale. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If workload is infeasible, reslice the bottleneck and state the failure; do not silently shrink the agreed eventual battle scale.

## Verdict — 2026-09-25

**Result: the scale verdict is delivered, and it is split.**
- **The live 100-a-side battle:** meets the frame and memory budgets. The simulation step misses its p95/p99 targets only while the full armies are fighting.
- **The late state (20,000 fallen, 2,000 wrecks):** fails the step, frame and readability targets on the current architecture.

Per this slice's feedback rule, the bottlenecks are resliced into [16b](16b-late-battle-scale.md), which awaits the user. Nothing was capped or erased to pass.

**What was built:**
- `sim::endurance` is the synthetic stress battle: seeded waves, reserves, a speculative-fire burst at minute 30, and an optional authored late state. `Battle::load()` reads the counters from their owning stores.
- `cargo run -p sim --release --example endurance_report [minutes] [late]` is the accelerated soak.
- `/lab/endurance` shows live telemetry: frame intervals over 10 s, main-thread heap, GPU buffer bytes, and blue's units and seen remains. Its scene runs on a **production build** (a new `"build": "production"` registry flag), because React's dev build clones every prop into the performance timeline.
- The village route's view became the shared `BattleView`.
- Outcome-identical speed-ups (digests unchanged on the village and the late state):
  - a height-only terrain query;
  - fog rays stop once nothing further can be seen, and skip `exp` with no foliage;
  - sensing lists the living enemy once per side;
  - `separation` computes its own radius once.

  Together they took one fog sweep from 32 to 23 ms at 100 units a side. `sensing::validate` pins the positive attenuation those rays rely on.

**Accelerated soak** (native release; macOS arm64, M5 Pro; seed 1):

| run | tick p50 | p95 | p99 | max | notes |
|---|---|---|---|---|---|
| 60 simulated minutes, whole run | 3.4 ms | 23.8 | 36.8 | 1088 | 476,760 rounds; remains kept (850 fallen, 40 wrecks at the end); RSS 125–159 MiB |
| minutes 0–10 (full armies) | 10.7–15.8 | 37.5–38.1 | 40.4–42.9 | 1088 | about 1.1 s stall on each 160-unit order wave |
| minutes 20–60 | 2.0–3.6 | 5.8–9.8 | 5.9–14.7 | 33 | the battle burns out: 39 units alive from minute 40 |
| late state, 10 minutes | 28.4 | 57.4 | 62.7 | 966 | 20,317 fallen, 2,020 wrecks; RSS 135–168 MiB |

Peak rounds in flight: 444. Peak rounds launched per simulated second: 499.

**Real-time rendered runs** (production build; Chromium 148; Apple Metal 3; 1280×800, DPR 1):

| run | sim rate | frame p95 | frame p99 | memory |
|---|---|---|---|---|
| live battle, 300 s | 26.5 Hz | 9.8 ms | 10.3 ms | main-thread heap 91 MiB; GPU 23 MiB |
| late state, 300 s | 6.4 Hz | 442 ms | 442 ms | whole page with worker 805 MiB; GPU 150 MiB |

**Against validation.md's targets:**
- **Simulation step, p95 under 20 ms and p99 under 30 ms:** met from minute 15 on. Missed while the armies fight (p95 38 / p99 43 ms, plus a 1 s wave stall) and in the late state (57/63 ms).
- **Frames, p95 under 25 ms and p99 under 50 ms:** met live (9.8/10.3 ms). Missed in the late state (442 ms).
- **Memory, 1.5 GiB:** met in both.
- **No catch-up spiral:** met. The worker sheds tick debt and flags "behind real time" (slice 03), so a wave stall costs one late tick, not a burst.
- **Rounds per second:** the roster peaked at 499, far below the 6k–8k burst named in validation.md. 100 rifle squads at the fixture's rates cannot reach it; 16b asks where that target belongs.

**Checks:**
- `crates/sim/tests/endurance.rs`: the roster shape and a repeatable scenario; the late state starts with its remains and never loses one.
- Scene `endurance`, 3 checks:
  - the battle runs in real time without failing;
  - three resets return GPU buffers and textures to the same count;
  - the late state runs without failing.
- Withheld credit, a hidden tab and reset are covered by the `authority` scene (slice 03).

**Visual gate:** an unprimed critique of the early, late-run and late-state frames.
- **Blocker, recorded in 16b:** in the late state, remains bury the living. Fallen soldiers are drawn at the size and near the tint of living ones.
- **Blocker, recorded in 16b:** at about 6 Hz the late fight barely advances.
- **Major, recorded in 16b:**
  - At the 2.4 km overview, units are too small to find.
  - The authored wrecks are static map props, which every side knows, so they show inside fog.
- **Major, kept:** contacts and the hold ring have no legend (the lab convention).
- **Minor, kept:** telemetry lines wrap.
- Preview-shots: not offered; the run is unattended.

**Code review, acted on:**
- The late run now lasts as long as the live run (`ENDURANCE_LATE_S`).
- A production fixture refuses `VERIFY_URL`; a server that fails to start fails only its fixture; servers close with `allSettled`.
- Reset now clears the session's last frame, so the reset check waits for a real restart.
- The endurance view keys on the scenario it was built from, and shows build errors. The seed is validated.
- The heap is labelled main-thread only; the whole-page figure is measured separately.
- The frame window is 10 s of wall time.
- Remains use their own random stream, so the late state never moves the waves.
- The report labels read "projectiles launched per sim-second".

**Harness fixes found by the full suite:**
- A dev server started earlier in the same run had set `NODE_ENV=development`, so the "production" build shipped development React and the late state crashed. The runner now pins `NODE_ENV=production` for that build. The verdict's numbers came from standalone runs, which were unaffected.
- No scene can hang the gate any more. Each scene has `SCENE_TIMEOUT_S` (900 s by default), and a crashed page is reported as a failure.
