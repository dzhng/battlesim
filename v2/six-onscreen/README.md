# Six jets on screen: supplemental perf sample (jet FX v2, `32d29c1e8`)

The six-jet bench in `evidence/v2/bench` replays the sixturn run through its original camera. That camera
shows all six aircraft for only part of the run, so its numbers undercount the cost of fragments when every
jet's FX is on screen. This sample replays the **same recorded sim** with a camera that keeps all six jets in
frame for the whole timed interval. It does not replace the original numbers.

## Result

Native, 1920×1080, the same pinned binary for off and on (sha256 `142445e8…`):

- **off** = presentation.json at HEAD without `effects.jet_exhaust`;
- **on** = presentation.json at HEAD (`32d29c1e8` data).

The runs alternate off/on over 5 pairs and both GPU locks were held. Each run has **284 timed frames**:
the bench's first 60 draws are warm-up and the last draw has no interval. That gives **1420 frames per
side**. Every frame is kept and no run was dropped.

| ms per frame | off median | on median | Δ median | off p95 | on p95 | Δ p95 |
|---|---|---|---|---|---|---|
| GPU (render-pass timestamps), pooled | 14.023 | 14.075 | **+0.052** | 17.125 | 17.170 | +0.045 |
| CPU wall (frame+prepare+encode+submit), pooled | 6.779 | 6.753 | −0.027 | 8.287 | 8.274 | −0.013 |

**Per pair (on − off):**

- GPU median Δ: +0.033, +0.105, +0.053, +0.047, +0.027 ms. The median is **+0.047** and all 5 pairs are
  positive.
- GPU p95 Δ: from +0.005 to +0.074 ms.
- CPU median Δ: +0.089, −0.237, +0.130, −0.099, +0.044 ms. This is noise.

**Spread of run medians:**

- GPU: off 0.051 ms, on 0.055 ms.
- CPU: off 0.319 ms, on 0.204 ms.

The **GPU cost of the FX with all six jets on screen is about +0.05 ms per frame (~0.35%)**. The original
bench measured +0.018 ms with only part of the formation in view. **CPU has no cost that can be resolved**
at a run-to-run spread of 0.2–0.3 ms. The bench's draw counters show what was drawn:

- on: an average of 952 effect instances per frame;
- off: 6 effect instances per frame;
- both: 6 vehicles per frame.

**Conditions:**

- No rustc/cargo ran during the bench. It started after the final check chain wrote `DONE`, at 20:38 Dubai.
- Long-running interactive Google Chrome helper processes were present during every run. They were the same
  for off and on (`.jet-fx-local/v2/six-onscreen/logs/ps-*.txt`).
- The bench environment was `BATTLEGAME_HIDE_GEAR=1 BATTLEGAME_AIRFRAME_LIGHT=0.15,0.1,65`, the same as the
  original bench's capture look. The original also set `CHASE_LEAD_S`, which only steers clip framing. Bench
  mode draws each recorded frame at its own camera and sim time.

## Framing: what changed and what did not

The camera lives only in the recording's frame records (`eye`, `target`, `up`), so no source edit or rebuild
was needed. The recording was copied to `.jet-fx-local/v2/six-onscreen/run/market-town/` and changed as
follows:

- **Unchanged.** Every byte before the `"frames"` key: schema, workload, tickHz, warmTick, side, layout and
  all 332 publications. Its sha256 is `c579f40d…` in both the original and the copy.
- **Changed.** In the 345 frame records, only `eye`, `target` and `up`. `tick`, `simTime`, `reelMs`, `canvas`
  and `plate` are untouched.
- **Manifest.** The copy differs only in `identity.captureHash`, which the native loader checks:
  `6ed5e420…` became `ac8f7570…`.
- **Original.** The run in `throwaway/native-parity/2026-11-10T15-56-01-239Z/` was verified unchanged
  (`original-sha256-before.txt`).
- **Not faked.** Jet motion and FX come from the recorded publications and the real presentation. Nothing
  about them is scripted.

**The rig** follows the formation's centre with a smoothed chase:

- distance 220 m;
- look-down 0.35 rad;
- 0.8 rad toward the rear-left, which is the inside of this left turn, so banked jets show their planform;
- heading smoothed with τ 0.9 s.

**Where the positions come from.** The jet positions that aim the camera were decoded from the recorded
publications with the browser presentation wasm (`decode.mjs` → `decoded.jsonl`). They match the sim probe
to 0.0 m.

**How the framing was checked.**

- *Analytic, every frame:* in every timed record (60..343, sim 5.94–11.04 s), each jet plus a 12 m margin
  projects inside the frame. The worst |NDC| is 0.79 and the jets are 137–270 m from the camera
  (`framing-report.json`, per record).
- *Visual:* frames were rendered every 250 ms across the window (21 frames). I marked each jet's projected
  published position and checked by eye that every frame shows six drawn jets under the six marks
  (`contact-sheet-marked-21.png`).

**The window covers the hard pull and the release.** The F-22's load factor, from published motion, runs as
follows:

- n ≈ 5.7 at 6.2 s;
- 11.3 at 7.2 s;
- 8.9 at 8.2 s;
- 6.0 at 8.9 s;
- 3.6 at 9.9 s;
- 2.4 at 10.9 s.

So most of the timed frames carry heavy wingtip vapour.

## Files

| File | What |
|---|---|
| `contact-sheet.png` | Six times across the timed window, feature OFF \| ON, same camera and sim (labelled sim/reel/record/load) |
| `contact-sheet-marked-21.png` | All 21 witness frames (ON), each jet's published position circled |
| `still-hard-pull-{on,off}.png` | Full-size frame at sim 7.19 s (n ≈ 11): six banked jets with vapour |
| `bench/{off,on}/<pair>/*.json` | Raw native bench reports: per-frame `gpuMs`, `cpuMs`, phases |
| `summary.json` | Per-run and pooled median/mean/p95, run-median spread, per-pair and pooled deltas, framing provenance |
| `framing-report.json` | Hashes, rig, per-record worst NDC and jet distances |
| `contact-sheet-rows.json` | The six contact-sheet times |

## Limitations

- **The sample is one shot.** It is one formation, one turn, and one camera distance on Market Town, over
  5.1 s of sim time. It is not a broader workload.
- **Jet size.** The jets cover roughly 60–150 px across, depending on bank and distance.
  - In the release frames they are side-on and low-contrast against the town, though present in the frame.
  - A closer camera would raise fragment cost further. This framing is a representative chase, not a
    close-up.
- **The two benches are not directly comparable.** Absolute frame times differ from the original bench
  because the camera is different (GPU median 14.0 vs 14.5 ms). Compare deltas within each bench, not
  absolute times across them.
- **Wall time is not reported.** "CPU wall" is the bench's per-frame CPU time with GPU waits excluded.
  Interval and FPS figures, which are paced by the GPU, are in the raw files but not summarised.

## Reproduce (from the worktree root, local tooling in `.jet-fx-local/v2/six-onscreen/`)

1. `bun decode.mjs <web/src/wasm/renderer_browser> <original capture.json> decoded.jsonl`
2. `python3 author.py <original run>/market-town run/market-town <probe.jsonl> framing-report.json`
   (rig defaults in the file)
3. `run.sh frames <off|on> 1100 6220 4 <dir>`, then `overlay.py` / `sheet.py` for the sheets.
4. `run.sh bench 5` (both GPU locks; local `roots/{off,on}` = v2 roots plus the saved
   `jets-fx-sixturn` encounter copy, which was removed from the tracked tree when the helper was restored),
   then `summarize.py evidence/v2/six-onscreen/bench framing-report.json evidence/v2/six-onscreen/summary.json`.

Published media contains the witnesses, framing reports and summary. Raw benchmark JSON and local reproduction scripts are retained in the review artifact bundle, not the feature diff.
