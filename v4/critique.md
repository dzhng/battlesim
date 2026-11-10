# v4 engine trail looks — FOR PR REVIEW (not fully cinematic-approved)

**Source commit:** `8eaf7a59b`, on top of the published `017773c74`. The coordinator accepted the plume3 render for human review on PR #33, with the limitations below. **This is not a claim of full cinematic or photoreal success.**

## Latest review and scope decision (Nov 11, Dubai)
The reviewed A/B closeups were accepted as clearly visible and distinct; both are frozen unchanged. The earlier independent B borderline note below is retained as a phone-strip caveat, not a request for another A/B pass. C is retained under the explicit optional-change allowance: soft, genuinely puffier billowing is not a safe tiny adjustment to this already-tested ribbon path. Its cow-print/mottled character remains an acknowledged weakness; no further proof or claim of improved C.

## Request
- **Asked for:** more noticeable engine exhaust and a black/white smoke mix, judged at phone size.
- **Targets:**
  - A: a clearly white trail.
  - B: a white trail with a continuous dark-grey core that thickens on the afterburner.
  - C: dense mixed black-and-white smoke.

## Verdict (this render: `variants-strip.png` with 360 px panels, and `variants-closeup.png`)

| Look | Key value | Verdict |
|---|---|---|
| **A (default)** | `realistic` | **Pass.** A warm haze into two clearly white streams with soft, fraying edges that fade out. Near the nozzles the streams are near-straight, as clean condensation is, but not an opaque pipe. |
| **B** | `white_core` | **Borderline.** A continuous dark-grey core inside white. On the graded frames the core averages 117 grey levels at 2.8 s and 134 at 6.1 s, against about 215–230 for A at the same pixels. At phone size it is only moderately distinct from A, and its core reads as grey streaks. The trail starts with a blunt front a little behind the nozzles. |
| **C** | `dramatic` | **Colour passes, shape borderline.** Unmistakably black and white, with no discs, beads, rings, rectangles or black tube. It reads as a mottled "camouflage" mass, still ribbon-like rather than true 3-D smoke, and starts with a blunt, detached front behind the nozzles. Dramatic only: not what a clean modern fighter makes. |

**Recommendation:** ship A as the default. C is only a dramatic option. B helps when engines need to read at a distance, but it is the weakest distinct look.

**Likely cause of the blunt fronts** (not changed): B's white layer starts 1.6 m wide and C's layers start 1.2–1.8 m wide, so the newest stretch ends in a wide square cap. A starts at 0.8 m and doesn't show it.

**Proof rounds:** 8 in total. Five came before the user resumed (texture puffs, denser puffs, jitter plus base ribbon, data-only correction, procedural billows), all rejected. Three came in this resumption (continuous plume; per-end width plus the zoom fix; the bounds fix, real breakup and a mixed material). The last one is this render. The rejected proofs are kept under `../proof-*` and in `../v4-closeout-UNPUBLISHED.tar.gz`.

## What was wrong, and what fixed it (proven, not assumed)
1. **Source freshness.** The pinned binary embeds the new shader code, built in through `include_str!(OUT_DIR)`. The earlier failures really were the new code, not a stale build.
2. **Round sprites.** The sheet-sprite and billow puffs had round outlines, so they read as beads, rings and discs. They were replaced by continuous plume ribbons: the middle wanders, the width swells, and the inside is broken by noise fixed where the smoke was laid, down to clear gaps.
3. **Staircases.** Each 33 ms stretch was drawn at one width while young width grows with √age, about 15% per stretch. A plume now carries each end's own width, and a correction undoes the warp of a tapered quad.
4. **Edges cut straight.** The soft body reached 1.25 of the half-width and was clipped by the quad. It now stays inside it.
5. **Pale black.** Partly see-through black over an HDR sky stays pale. Plumes now hide by optical depth (`alpha = 1 − e^-depth·cover`). C is one mixed material, its grey running from dark to light by noise, rather than white stacked over black.
6. **Zoom glow.** A plume's `misc.w` was being read as a strategic-zoom brightness boost below a pixel width. That is fixed for plumes only: when thinner than a pixel they hide less instead of glowing.

## Unchanged (invariants)
- Wingtip wisps: their look, data and trigger (g-load, thresholds, hysteresis, attack and release).
- Glow, afterburner flame, nozzle positions, and the A-10 having no flame.
- The simulation.
- The wisp shader block, the ribbon, wisp, flipbook and glow batch functions, and the record layout. New engine paths are flagged in `misc.w`.

**No heat shimmer:** the near-nozzle haze is drawn smoke, not refraction.

## Performance (FX off against C, native, Apple M6, 1920×1080)
**Setup:**
- **Binary:** pinned `b55d3da5…`, built from the same source as `8eaf7a59b` before a test-only file split.
- **Scene:** six-jet on-screen replay, all six jets in frame (worst NDC 0.785 including a 12 m margin).
- **Runs:** 5 alternating pairs × 284 timed frames after a 60-frame warm-up, 1420 frames a side. Both GPU locks were held and nothing else ran. Every run measured all 284 GPU timestamps.

| | FX off | C | Δ pooled | Per-pair Δ |
|---|---|---|---|---|
| GPU median | 14.022 ms | 14.281 ms | **+0.259 ms** | +0.231 … +0.276 |
| GPU p95 | 17.137 ms | 17.392 ms | +0.254 ms | +0.185 … +0.307 |
| CPU wall median (excl. GPU waits) | 6.705 ms | 6.821 ms | +0.116 ms | +0.043 … +0.181 |
| Offscreen frame interval, median / p95 | 22.47 / 26.99 ms | 22.81 / 27.28 ms | | |
| Effect instances drawn (`conditions.meanDrawn.effects`) | 6 | 2132 | | identical in all 5 runs |
| Vehicles drawn | 6 | 6 | | |

- **Old figures superseded:** this replaces the rejected-prototype figure of +0.348 ms.
- **Not benchmarked:** A and B. C is not necessarily the heaviest look: A carries a third, billow layer.

## Checks
Native release build, WASM build, presentation suite, and the normal source commit hook (fmt/workspace clippy/file-size checks) passed. Full workspace and native/renderer test suites are still running; this is NOT an all-checks-green claim. Their final status will be added to PR #33 and CHECKS.md.
