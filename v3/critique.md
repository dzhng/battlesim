# Jet FX v3: short wispy wingtips, plus three switchable engine trail looks

Branch `feat/jet-engine-glow-contrails`. These are new commits on top of the published `32d29c1e8`:

| Commit | What |
|---|---|
| `ccdb51ce8` | Wingtip vapour drawn as short, tapering wisps broken by noise, anchored where it formed (shader + record) |
| `941565a12` | Wisp look data from the three wingtip proof rounds |
| `b67c130ad` | Switchable engine trail looks: `engine_style` / `engine_styles`, layer `reheat`; wingtip breakup 1.0 |
| `7cfcdb707` | Engine look data that reads from a close chase (one correction after the first A/B/C render) |
| `017773c74` | Fix: trails fade in smoothly instead of starting square-cut |

## The three engine looks (config key, no rebuild)

`presentation.effects.jet_exhaust.engine_style` picks one of `jet_exhaust.engine_styles`:

| Value | Look | Use |
|---|---|---|
| `"realistic"` (**default**) | Short grey-brown haze off the nozzles, fading within about 18 m into a clearly visible, soft white condensation trail | Recommended |
| `"white_core"` | Wider, brighter white trail around a darker grey core that becomes denser with the speed/cruise throttle proxy and afterburner | More visible engines; a stylised look |
| `"dramatic"` | Dense near-black core layered with white | Artistic only: persistent heavy black smoke is not typical of a healthy modern fighter |

**How the looks work:**
- **Layers:** each look is a list of trail layers, laid from the same per-tick points behind every nozzle
  and drawn in order. Layers reuse the nozzle's retained sample ring; they add batched ribbons, not a separate history, pass or texture.
- **Throttle and afterburner:** throttle is the existing proxy, published speed against cruise. A layer's
  `reheat` adds opacity from the presentation-only afterburner proxy, read as before from speed past cruise.
- **Same everywhere:** native and browser share the same presentation and sprite shader.
- **Unchanged:** glow, the afterburner flame, and the A-10 having no flame.
- **"Haze" means drawn smoke.** It is not refraction: nothing bends the picture behind a jet. Real heat
  shimmer would need a screen-space distortion pass, and there isn't one.

**Recommendation: ship A (`realistic`).**
- A is the least sooty and most transparent of the three, making it the most sensible default for a modern fighter. It is still an art-directed approximation, not fully convincing gas at every angle.
- B looks like a solid white beam when seen close.
- C looks like a damaged aircraft. That suits a cinematic choice, not the default.

## Wingtips

**What changed:**
- The wisps are about a quarter of v2's length: 0.3 s, against 1.1 s.
- They are thinner at the tip, taper sharply, and are broken by noise anchored where the vapour formed.
- `b67c130ad` raised the breakup from 0.75 to 1.0, so gaps can open fully. The old value kept a quarter
  of the opacity in every gap.

**What did not change** (pinned by a test):
- the g-load read from motion;
- the thresholds 1.8 / 4.0, hold until 1.5;
- the onset, the attack of 0.25 s and the release of 1.0 s;
- the `vapour` config.

The load, vapour and heat code is byte-identical to `32d29c1e8`. Its sha256 is `c60e3d65…`, and the
trigger-guard check confirms it.

## Evidence (`.jet-fx-local/evidence/v3/`)

All of it is real renders of the same real sim turn (F-22, 800 km/h, level → about 60° turn at up to
about 11.9 g → level). The camera and timestamps are identical for every look.

- **Camera:** a camera-only copy of the recorded run (`v3/hero/run`). Only the frame cameras change, and
  every publication is byte-identical. It is a chase 34 m from the jet, on the inside of the turn, with the
  jet about 35% of the frame width.
- **Binary:** pinned native `b7108421…` (code `017773c74` plus the capture-only helper), with frozen roots
  per look.

| File | What |
|---|---|
| `variants-strip.png` | A / B / C columns at 0.6, 1.6, 2.8, 4.3, 6.1 s, labelled; includes the 4.3 s weak moment |
| `variant-A.mp4`, `variant-B.mp4`, `variant-C.mp4` | The full 6.2 s shot in each look, 1280 px |
| `turn-strip.png`, `turn-clip-6s.mp4`, `turn-phone-strip.png` | Default A: level (no wingtip vapour) → wisps → release → steady |
| `level-no-wingtip.png` | Default A in level flight: glow, engine trail, no wingtip vapour |
| `crop-steady-ABC.png`, `crop-turn-wingtip-ABC.png`, `crop-easing-out-4.3s-ABC.png` | Full-resolution crops, A \| B \| C |
| `frames-A/`, `frames-B/`, `frames-C/` | Full frames retained in local Mini/box evidence only, not uploaded to the media branch |
| `six-onscreen-C/` | Benchmark, whole feature off vs C (below) |
| `review-wisps-only/` | Local archive of earlier wisp-only proofs; not on the media branch |
| `SUPERSEDED-*` | Local Mini archives only, not published or used: pre-fix or overlapped benchmark runs |

## Review rounds

1. **Wingtip proofs 1–3** (before the engine looks): proof 1 was invisible, proof 2 visible wisps, proof
   3 shorter. The critique still saw beams to the frame edge.
2. **First integrated A/B/C render:** all three engine trails were too faint to tell apart, and A's white
   trail began off frame. This led to the one data correction, `7cfcdb707`.
3. **Second A/B/C render, with an unprimed critique.** It picked A as the default. Its sharpest finding:
   in steady flight every look's trail began behind the jet with a square-cut front edge, as a solid tube.

   Root cause: each trail stretch spans one published tick (about 7 m) and holds one opacity, so a short
   fade-in rose in one or two flat steps. The fix, `017773c74`, draws still-thickening stretches in four
   parts, each at its own age. A regression test covers it, and mutation-checked: with one part it fails.
4. **Final render:** the trails now thicken smoothly from just behind the nozzles.

## Honest critique of the final looks

- **A:** a softer, lower-density white trail that starts near the nozzles, but still has a ribbon/tube character in close steady flight. The grey-brown haze is subtle; at
  this distance it is hard to see on its own.
- **B:** distinct from A at full size: brighter, wider, with a darker core that grows on the afterburner.
  At strip size A and B are close.
- **C:** clearly darker and smokier. That is by design, and not realistic.
- **Steady-flight trails** stay fairly even along the first 50 m in view. Their taper acts over their full
  2–3 s life, which is hundreds of metres. The two engines' trails overlap into one band from the side.
- **Wingtip at 4.3 s:** the lower tip's wisp still reads as a long, faint line to the frame edge. The sim
  jet is really pulling about 6 g there (probe n ≈ 6.3 at 9.2 s sim) while the drawn bank has mostly
  rolled out. By requirement, neither the trigger nor the g-load is changed, and nothing is retimed.
  Full breakup (1.0) helped the gaps but didn't remove the line.
- **The hard turn:** all three engine trails become a loose, patchy haze, and are hard to tell apart there.
- **Other limits:**
  - At a grazing angle the upper wingtip's trail can read as a hairline along the horizon.
  - The 6.1 s frame has a road under the jet that can read as a trail at phone size.
- **Not added:** heat shimmer (needs a distortion pass), and wing-surface condensation (no wing-surface
  anchor in the baked jets, and a cheap sprite would risk a fake permanent cone).

## Performance (heaviest look, C)

Six jets pulling together, native, Apple M6, 1920×1080, with the heaviest look, C (`dramatic`).

**Setup:**
- **Scene:** v2's six-on-screen replay. It is a camera-only copy of the six-jet turn run, with
  publications byte-identical, and all six jets in frame for every timed frame (worst NDC 0.785 including
  a 12 m margin).
- **Comparator:** the **whole feature off** (`jet_exhaust` removed) against **final C**. Both use the same
  pinned binary `b7108421…` and frozen data.
- **Runs:** 5 alternating off/C pairs × 284 timed frames after a 60-frame warm-up, 1420 frames a side.
  Both GPU locks were held; per-run snapshots showed no concurrent build, render or video-encoding job. An earlier run that overlapped video encoding, and one
  on the pre-fix binary, are kept as `SUPERSEDED-*` and are not used.

| | feature off | C | Δ (pooled) |
|---|---|---|---|
| GPU frame median (pass timestamps) | 14.193 ms | 14.286 ms | **+0.092 ms** |
| GPU p95 | 17.316 ms | 17.457 ms | +0.141 ms |
| CPU wall median (framing+prepare+encode+submit, excl. GPU waits) | 7.839 ms | 7.917 ms | +0.077 ms |
| CPU p95 | 9.903 ms | 10.006 ms | +0.103 ms |

- **GPU:** every pair is positive, from +0.022 to +0.103 ms (median +0.082 ms). The run medians spread
  from 14.06 to 14.48 ms (off) and 14.11 to 14.54 ms (C).
- **CPU:** the per-pair median deltas range from −0.09 to +0.24 ms, so the CPU change is within noise.
- **Reading:** C costs about 0.1 ms of GPU for six jets, from two wide layers per nozzle that last up to
  3 s. A and B have shorter or narrower long layers, so they should cost no more, but they were not
  benchmarked separately.

Raw reports: `six-onscreen-C/bench/{off,C}/<pair>/`; summary: `six-onscreen-C/summary.json`.

## Checks

Final chain on HEAD `017773c74`. Logs: `.jet-fx-local/logs/v3-engine-final*.log`.

| Check | Result |
|---|---|
| `bun run build:wasm` | pass |
| `cargo build --release -p native-client` | pass |
| `cargo test -p presentation` | 44/44 test binaries pass, including `tests/jet_exhaust.rs` 21/21 |
| `cargo fmt --check`; workspace `cargo clippy --all-targets -D warnings`; size ratchet | pass (the clippy and ratchet results are from the pre-commit hook on each of the five commits) |
| Focused GPU tests (`effect_wisps`, `high_air`, `pipelines`, `pipeline_abi`) | pass at `ccdb51ce8`; renderer-core is unchanged since |
| Rest of the workspace (non-GPU) | **running** (`v3-engine-final-workspace.log`) |
| GPU suites, `renderer-core` + `native-client` `--no-fail-fast` | **queued after the workspace step** (`v3-engine-final-gpu.log`). `native-client --test counts` is expected to fail on the pre-existing stale fixture (`1a8123141`) |

`jet_exhaust.rs` covers:
- **The 17 earlier tests,** kept.
- **Look against envelope:** a flight drawn under the plain and the wispy wingtip look forms and fades
  vapour identically.
- **Wisps:** they stay where they were laid, with no seams, and engine trails stay plain.
- **Engine looks:** the `engine_style` in the data alone selects the layers, and nothing else changes.
- **Reheat:** a layer thickens on the afterburner only by its `reheat`.
- **Shipped looks:** every one loads when chosen.
- **Fade-in:** a trail thickens from the nozzle without a tick-long step.

Mutation checks:
- coupling the look to the strength fails the envelope test;
- removing the shader breakup fails the GPU wisp test;
- `FADE_IN_PARTS = 1` fails the fade-in test;
- the earlier hysteresis and release mutations still fail.

## Independent executor visual review

I inspected the final A/B/C full-resolution steady and wingtip crops, and sampled all three 6.2-second clips at 3 fps (19 frames each). The soft-start correction removes the obvious square-cut front edge. It does **not** make the steady engine plume volumetric: it still reads as an even band/tube, especially B. A/B are subtle at strip size; C is visibly greyer/sootier and can read as damage.

The wingtip material is shorter, thinner and mottled, and level flight has no wingtip emission. At grazing moments around 3.7–4.3 seconds a wingtip still projects as a long straight white streak; I would not call that fully photoreal. The g-load timing was deliberately not changed to hide it. White condensation here is art-directed, not conditioned on altitude, humidity or temperature. True optical heat shimmer and 3-D turbulent smoke are not implemented.

The replay/camera/timestamps match across A/B/C, but the procedural wisp pattern is not pixel-identical: retained-history length can change its distance-phase origin. This is not a different wingtip trigger or an afterburner-driven wingtip effect.
