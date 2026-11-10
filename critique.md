# Jet engine glow + trails: evidence and honest critique

Branch `feat/jet-engine-glow-contrails`: commits `575a142a0` (feature) and `583fdaf45` (trails laid per
published tick; tuning from these renders), off `staging` at `673b68ce8`.

## How the evidence was made (same frame, honest A/B)

- **Scenes:** capture/jets-showcase-v4.1's Market Town city chase (`9ae75c7ec`, read with `git show` only).
  Its capture-tool chase rig, gear-up, airframe light and chase lead were applied as **uncommitted local
  changes** (`.jet-fx-local/capture/helper.patch`) and restored afterwards. None of it is in the feature
  commits.
  - `jets-v41-3`: F-22A, orbit to dead astern (rear chase).
  - `jets-v41-1`: A-10C.
  - `jets-v4fx-ab`: local. v4.1's Typhoon with the **sim's own afterburner** ordered at tick 0 (1167 km/h,
    1.49× its 780 km/h cruise). It spawns 615 m further back so it is still over the town, and is chased
    from a rear side quarter.
  - `jets-v4fx-six`: local. All six airframes abreast at 760 km/h over the town. Used for the benchmark.
- **Pipeline:** one browser parity run per scene records publications and cameras. One pinned native
  binary replays it offscreen at 1920×1080 (`.jet-fx-local/bin/native-client`, built from `583fdaf45` +
  helper; sha256 `ac7e1b6f…` for the final round).
- **Before/after:** the **same binary, run and reel time**, with two repository roots that differ only in
  `fixtures/presentation.json`. "before" has no `effects.jet_exhaust`, which is the feature off; the code
  path without that section draws nothing extra. `pairs/*-diff-x4.png` confirm that only exhaust pixels
  change (±1-level noise elsewhere).

## Files

| What | Where |
|---|---|
| Raw stills, before/after | `frames/<scene>-<reel ms>-{before,after}.png` |
| Side-by-side pairs, full frames | `pairs/<scene>-<ms>-full.png`; diffs ×4 in `pairs/*-diff-x4.png` |
| Rear chase over city (F-22A) | `pairs/rear-chase-f22-crop.png`, `pairs/jets-v41-3-5400-full.png` |
| Side-quarter afterburner (Typhoon) | `pairs/side-quarter-afterburner-typhoon-crop.png`, `pairs/jets-v4fx-ab-{300,2750}-full.png` |
| A-10C (front quarter / rear) | `pairs/a10-front-quarter-crop.png`, `pairs/a10-rear-crop.png` |
| Six jets | `pairs/jets-v4fx-six-{3000,5400}-full.png` |
| Phone size (1170 px wide) | `phone-chase-frame.png`, `phone-strip.png` (F-22 rear, Typhoon reheat, A-10 rear) |
| 5 s clip (after only, 30 fps, no motion blur) | `afterburner-typhoon-5s.mp4`, `clip-contact-sheet.png` |
| Benchmark raw reports + summary | `bench/{before,after}/<run>/native-market-town-*.json`, `bench-summary.json`; superseded round-2 data in `bench-round2/` |

## Proof rounds (3 of the allowed 4)

1. **Round 1 (frame-history trails).** The first six-jet stills showed glows but **no trails at all**. A
   still framed at reel 5400 ms draws one frame, so there was no history. This was a real design bug (any
   replay seek or clip start shows the same thing). Trails were redesigned to be laid **per published
   tick**, as damage smoke is (`583fdaf45`), with a regression test.
2. **Round 2.** The F-22 glow was a dim orange tint and the trails barely visible. The Typhoon's reheat
   blew out both nozzles into a white blob. The A-10 glow leaked past its nacelle from the front.
   Retuned.
3. **Round 3 (shipped).** An unprimed critique of round 2 (it saw only the after frames) said:
   - **Trails looked detached from the aircraft.** A 0.12 s fade-in hid the first ~27 m.
   - **The A-10 glow was too bright** for a TF34.
   - **Trails read as milky ground fog**: up to 7 m wide.
   - **The F-22 glow was a flat dim rectangle.** I had authored it at the bore recessed 0.7 m inside the
     2D nozzle, so the soft-depth fade dimmed it.

   All four fixed: fade-in 0.03 s / 0.015 s, the A-10 glow 0.45 and redder, trails 0.5→3 m (engine) and
   0.25→1.4 m (wingtip), and the F-22 authored at its exit plane.

## Visual critique of the shipped round (my own, after the fixes)

- **F-22 rear chase:** good. Two hot amber glows bloom in the 2D nozzles. Four trails leave the right
  places: thin white wingtip vortices from the tips and faint grey haze from the nozzles, widening toward
  the camera. It reads on the phone strip. The engine haze is deliberately faint (at 80 m AGL a real jet
  makes no contrail). It is the weakest element at phone size.
- **Typhoon afterburner:** the strongest effect. An amber plume with a white-hot core starting just aft
  of the lips. It reads at phone size and through the whole 5 s clip.
  - From this rear-side angle the plume still overlaps the nozzle exits. That is plausible (the flame is
    in front of the exit from there), but it hides the nozzle lip detail.
  - The shock diamonds are too subtle to see at this range.
  - The plume ends hard against the scenery: no heat shimmer (see gaps).
- **A-10:** dull red warmth deep in each fan exhaust from behind, invisible from the front, no flame.
  Wingtip vortices are visible. Arguably still a touch strong for level flight; this is one data value
  (`wingtip_trail.opacity[0]`).
- **Six jets:** at ~300 m all six glow and trail. Trails at that range are thin lines. Readable, not
  dramatic.
- **Framing caveat:** the afterburner shot's chase rig puts the jet high in frame, not centred. That is
  capture authoring in my local helper, not the feature.

## Performance (six jets, native, Apple M6, 1920×1080, same run, 5 alternating before/after runs × 209 timed frames after 60 warm-up)

| Metric | before | after | Δ |
|---|---|---|---|
| GPU frame (render-pass timestamps), median | 14.984 ms | 15.020 ms | **+0.036 ms (+0.24%)** |
| GPU p95 | 17.457 ms | 17.488 ms | +0.031 ms |
| CPU wall (framing + prepare + encode + submit; excludes GPU waits), median | 7.259 ms | 7.179 ms | −0.080 ms (noise) |
| CPU p95 | 9.420 ms | 8.498 ms | noise: before run 1 was an outlier, run medians 8.54 vs ~7.15 |
| CPU framing phase (presentation incl. jets), median | | | +0.008 ms |

- **Noise:** the spread of run medians is 0.03–0.04 ms on GPU. On CPU it is 0.16 ms (after) and 1.4 ms
  (before, with the outlier run).
- **Round 2 agreed:** GPU +0.057 ms, framing +0.009 ms, CPU −0.04 ms.
- **Conclusion:** the GPU cost is ~0.04–0.06 ms for six jets and the CPU cost is below measurement. Both
  come from ~1–2k extra effect instances in the existing pass.

## Checks

- **Feature tests:** `cargo test -p presentation` — all 44 test binaries green, including the new
  `tests/jet_exhaust.rs` (10 tests). Mutation-checked: on the first design, frame-rate
  dependent sampling and a removed jump break each failed their test. On the shipped design, removing
  seek materialisation and the engines-out rule each fails its test.
- **Lint and size:** workspace `cargo clippy --all-targets -D warnings`, `cargo fmt --all` and the size
  ratchet all passed in the pre-commit hook for both commits.
- **Builds and workspace tests:** see `../logs/` (`workspace-tests.log`, `gpu-tests.log`, `wasm-final.log`,
  `native-final.log`) and the summary in PR_BODY.md.

## Gaps (honest)

- **No heat shimmer:** it needs a distortion pass, which is not cheap in the current pipeline, so it was
  left out.
- **Throttle is drawn from published speed against cruise.** In the capture encounters jets fly exactly
  at their overridden cruise, so reheat shows only when the sim's afterburner flies them past it. An
  own-side afterburner toggle at the same speed would not light the flame.
- **Browser high-air plane (fixed in `2c7038589`, browser-verified):** the browser composited the jet's
  own plane over the effects, hiding nozzle cores and the reheat core from astern. The plane now redraws
  the effects over its models (`effectsHighPlane`). Evidence: `browser/{before,after}/*.png` (air lab,
  same ticks and cameras, pre-fix vs fixed wasm), `browser/pairs/sheet.png`, `browser/pairs/*-x4.png`.
  An unprimed critique judged the fixed build hotter and more convincing astern and in rear quarter,
  identical from the side, with no silhouette fringes; it noted a mild glow spill past the F-22 nozzle
  rims and the Typhoon's plume and trails now drawn over its gear legs (in front of them, as native).
  The browser cost of the second effect draw is unmeasured.
- **No orientation check on the glow:** the glow is a camera-facing sprite. From the front, depth hides
  most of it, but its halo can still peek round a nozzle edge.
- **Coverage:** edge-transit aircraft (drawn from the publication's transit records) have no exhaust.
  Other roster jets (Rafale, Gripen, MiG-29, F-15, F-16…) have no authored `airframes` entry yet. That
  is data only, measured the same way.
- **Nozzle points** were measured from the baked meshes (`.jet-fx-local/tools/*.py`), because baking
  keeps no nozzle node. There is no automated check that they stay on the art if a model is re-baked.
- **The A-10 carries the shared fixed-wing profile's `afterburner` capability in the sim.** Its exhaust
  look has no flame, so it still never burns visually; changing the sim was out of scope.
