# Jet FX v2: wingtip vapour only in hard pulls

Commits on `feat/jet-engine-glow-contrails`, after the published `2c7038589`:

- `e6ea6b398` feat(effects): wingtip vapour only while the wings pull hard
- `32d29c1e8` tune(effects): shorter, softer wingtip vapour; fainter cruise engine haze

This update makes wingtip vapour intermittent and manoeuvre-load driven.

## Answer: vapour comes from the load on the wings, not from the afterburner

In film and in real life, wingtip vortex vapour forms when the wings pull hard: a break turn, a pull-up, a
high-g pass. Pressure drops at the tips and moisture condenses. Afterburner alone does not do it. A jet
burning straight and level makes heat, not tip vapour. So:

- **Wingtips:** vapour only while the wings carry real load. It shows in hard turns and pull-ups, and it is
  heavier the harder and faster the pull. There is none in level flight, at any speed, with or without
  afterburner.
- **Afterburner:** flame and shock diamonds as before. The engine glow is always on. There is no
  independent wingtip trigger.
- **Cruise engine haze:** kept, made fainter and shorter (see the critique below).

## How it works (presentation only; the simulation is untouched)

- **The load comes from published motion.** Each published tick, a jet's load factor
  `n = |(a + g·ẑ)⊥v| / g` is read from its published positions over `speed_window_s` (0.5 s):
  - the acceleration across its path, plus the lift that holds it up, over its weight;
  - level flight gives exactly 1, a level turn gives `√(1 + (v·ω/g)²)`, and a pull-up gives `1 + a_up/g`;
  - the drawn bank and the afterburner flag play no part.
- **Hysteresis and onset.** Vapour forms at `n ≥ 1.8`, at an `onset` thickness of 0.45, and thickens to
  full by `n = 4`. Once formed, it holds until `n < 1.5`. This is what stops a load hovering near 1.8 from
  flickering, and the test proves it (below).
- **Envelope.** Vapour eases in over `attack_s` (0.25 s) and out over `release_s` (1.0 s), tick by tick,
  so it doesn't depend on frame rate. The value is stored on each published pose. Seek, replay, pause,
  rewind, jumps and downed jets therefore behave as before, and a replay framed mid-flight shows the same
  vapour. Laid segments age out over the trail's life (1.1 s).
- **Contract change.**
  - `jet_exhaust.vapour_load` becomes `jet_exhaust.vapour {load, off_load, onset, attack_s, release_s}`.
  - The wingtip trail's weakest opacity is now 0.
  - `DrawnJet::load` (the drawn bank) is removed.

## Evidence (`.jet-fx-local/evidence/v2/`)

The flight and vapour come from a real sim turn. Only the evidence camera is authored along the probed path.

- **The encounter:** Market Town, v4.1's F-22 at 800 km/h and 80 m AGL, flying level. Two move orders do
  the rest:
  - one turns it about 60° left at 5.5 s;
  - a third order straightens it out.
- **Load the sim produced:** probed in the sim with the presentation's formula.
  - n = 1.00 until 5.6 s;
  - it rises to 11.9 by 7.4 s and holds through the turn;
  - it is back to 1.0 by 9.5 s.

  (The sim turns jets at about 40°/s, so its turns are very hard. Any real turn in the game therefore
  blooms fully.)
- **The camera:** 13 chained 0.5 s chase shots along the probed path, about 55 m behind in a rear-right
  quarter. It was recorded by a browser parity run, and native frames were rendered at 1920×1080 with the
  pinned v2 binary (`v2/bin`, sha256 `142445e8…`).

| File | What |
|---|---|
| `turn-strip.png` | Six frames: level (no vapour) → rolling in (forms) → hard turn → still pulling → level, vapour thinning → steady (engine haze only) |
| `turn-phone-strip.png` | Four of them at 1170 px wide |
| `turn-clip-6s.mp4` | The whole 6.2 s shot, 30 fps, 1280 px (5.2 MB) |
| `level-no-wingtip.png` | Level flight: nozzle glow and faint engine haze, zero wingtip trails |
| `frames/turn-*.png` | The six full-size frames |
| `six-pulling.png` | The six-jet bench shot mid-turn |
| `bench/`, `bench-summary.json` | Initial partial-visibility benchmark (data from `32d29c1e8`); `bench-round1*` is the same run before the tuning commit |

## Proof rounds (2 of the allowed 3)

1. **Round 1 (`e6ea6b398` data).** The behaviour was right: no vapour level, it bloomed with the bank and
   faded after the roll-out. An unprimed critique, given only the frames, judged the 1.6 s bank frame the
   most cinematic. It found:
   - the tip trails read as long, uniform "laser lines" running off frame;
   - they were still strong after roll-out and smeared into a fog-like band;
   - the cruise engine haze settled over the rooftops like ground fog.
2. **Round 2 (`32d29c1e8`, shipped).** Data only, and the same render pipeline as round 1:
   - tip vapour life 2 → 1.1 s, peak opacity 0.6 → 0.45, end width 1.4 → 2.4 m, release 1.5 → 1.0 s;
   - engine haze opacity 0.16 → 0.09, life 3 → 2 s.

   Re-rendered and inspected: the fog band and rooftop haze are mostly gone, and the vapour reads softer and less like rigid lines.

## Honest critique of the shipped round

- **Works:** level flight shows no tip vapour, before and after the turn. The bloom starts as the jet rolls
  in, is thickest in the hard bank, and thins within about a second of the roll-out. The 1.6 s frame (bank
  with two tip trails leading to the jet) is the money shot.
- **Weak:**
  - The trails are still fairly uniform ribbons, not the broken, wispy vapour of real footage. A
    turbulence or breakup texture along the ribbon would help, and it needs a shader change.
  - From astern, in a long pull, the trails still run to the frame edge.
  - At 4.3 s the trails are still strong because the sim jet is still pulling about 4–6 g as it eases out.
    That is correct, but it reads as late.
  - The release lags the end of the pull by up to the load window (about 0.25 s) plus the release.
- **Not added:** wing-root or top-surface condensation (vapour cones or sheets). There is no node for the
  wing surface in the baked jets, and a convincing sheet needs its own lit, view-dependent shape. A quick
  sprite risked exactly the misleading permanent "vapour cone" we were asked to avoid. Left out.
- **Heat shimmer:** skipped, as before. It needs a screen-space distortion pass (sample the finished frame
  behind the exhaust with an offset). The current pipeline has no such pass, and adding one is a new
  render-target read for every frame. That is not "truly cheap".
- **Initial bench framing:** the first bench included partial-visibility frames. The six-on-screen supplement below fixes this; its formation chase is still not a close-up worst case.

## Current performance: six jets on screen throughout the timed interval

Native Apple M6, 1920×1080, five alternating off/on pairs, 60 warm-up frames excluded then 284 timed frames per run (1,420 per side). Same pinned binary, same replay, only `jet_exhaust` disabled/enabled. Camera follows the formation at 220 m; only camera vectors/hash were changed in a replay copy, while the original layout and all publications remain byte-identical. All six jets stay within frame (including a 12 m margin) over sim 5.94–11.04 s. The 21-frame marked witness and paired six-time sheet were personally inspected.

| Metric | off | v2 | delta |
|---|---:|---:|---:|
| GPU median | 14.023 ms | 14.075 ms | +0.052 ms |
| GPU p95 | 17.125 ms | 17.170 ms | +0.045 ms |
| CPU wall median, excludes GPU waits | 6.779 ms | 6.753 ms | −0.027 ms, noise |
| CPU p95 | 8.287 ms | 8.274 ms | −0.013 ms, noise |

GPU run-median spread: 0.051/0.055 ms; all five paired GPU deltas are positive (+0.027 to +0.105 ms, median +0.047 ms). CPU paired deltas range −0.237 to +0.130 ms, so no CPU improvement is claimed. The small GPU cost is more representative than the initial partial-visibility result. No Rust compilation overlapped the sample. Existing interactive Chrome helpers remained present in both conditions.

See `six-onscreen/README.md`, `summary.json`, paired/marked contact sheets and raw `bench/` reports on the box. This is one formation/turn/camera, with aircraft about 60–150 px across. A closer overlapping shot may cost more. Browser-specific performance is still unmeasured. Compare off/on within each camera, not absolute times across the two differently framed benches.

## Initial six-simulated-jet benchmark (partial visibility; historical)

**Setup:**
- Scene: the six v4.1 airframes abreast at 760 km/h, all turning about 90° at 5.5 s.
- Comparator: the same binary and the same browser run, with two roots that differ only in
  `fixtures/presentation.json`. "off" has no `jet_exhaust`, so the whole feature is off; "on" is the
  shipped v2 data.
- Runs: 5 alternating off/on runs, 284 timed frames each after the bench's warm-up, 1420 frames a side.

| Metric | off | on (v2) | Δ |
|---|---|---|---|
| GPU frame (render-pass timestamps), median | 14.466 ms | 14.484 ms | **+0.018 ms** |
| GPU p95 | 15.376 ms | 15.408 ms | +0.032 ms |
| CPU wall (framing + prepare + encode + submit, excludes GPU waits), median | 6.569 ms | 6.638 ms | +0.069 ms |
| CPU p95 | 7.650 ms | 7.707 ms | +0.057 ms |
| Framing phase median | 2.193 ms | 2.228 ms | +0.035 ms |

- **Noise:** the spread of run medians is 0.034 ms (off) and 0.038 ms (on) on GPU, and 0.048 ms and
  0.120 ms on CPU.
- **Reading:** the GPU and CPU deltas are both inside or near that noise.
- **Round 1 data** (longer trails): GPU median +0.004 ms and CPU median +0.035 ms, in
  `bench-round1-summary.json`. Its run-median spreads were 0.020 and 0.026 ms on GPU, so both rounds sit
  at noise level.
- **v1 for comparison:** +0.036 ms GPU for six jets flying straight with always-on tip trails.

## Checks (on clean HEAD `32d29c1e8`)

Logs: `.jet-fx-local/logs/v2-final*.log`.

- **Builds:** wasm and native release build pass.
- **Tests:**
  - `cargo test -p presentation`: 44/44 binaries pass.
  - Rest of the workspace (non-GPU): 65/65 pass.
  - GPU suites (`renderer-core` and `native-client`, `--no-fail-fast`): 38 binaries pass, 1 pre-existing
    failure (below).

- **`tests/jet_exhaust.rs`:** 15 tests. The new and changed ones:
  - level flight at three speeds, and straight afterburner acceleration, lay no tip vapour;
  - a hard turn blooms and a gentle one doesn't;
  - the same turn rate flown faster pulls more;
  - a pull-up out of level flight blooms;
  - a load wavering across the threshold holds steady against a held load;
  - after the pull, vapour thins over its release, and the result is identical at 24 and 144 fps;
  - the replay, pause and rewind tests now turn, so they also cover the tip trails.

  Mutation checks: removing the hysteresis fails the flicker test; removing the envelope fails the
  release test.
- **Workspace clippy `-D warnings` and fmt:** rerun on final head `32d29c1e8`, both pass (`v2-final-clippy.log`, `v2-final-fmt.log`). Size ratchet passed the normal commit hook. No hooks were skipped.
- **Pre-existing failure:** `native-client --test counts` fails with "the publication layout is not this
  writer's". It is caused by the fixture `paris-corner` counts, which predate `1a8123141` in staging. It
  is not changed or caused here.
