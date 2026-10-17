# 05 — CLI writes MP4s and the report

## Contract unlocked

`bun run --cwd web capture <file.json> [--height=1080|1440] [--fps=60] [--no-cards] [--output=dir]` produces:
- one CFR H.264/AAC MP4 per scene;
- a joined MP4 with a short `xfade`/`acrossfade` crossfade, when there's more than one scene;
- `<name>.report.json`.

## Seam

- **`web/capture/`** holds pure stages: `args`, `plan` (capture → scene jobs), `transcode` (ffmpeg argv), `join` (ffmpeg argv) and `report`. One thin runner launches Chrome (reuse `launchLocalChromium`), records each scene through the route, then runs ffmpeg.
- **Report:**

  ```jsonc
  {
    "capture": "fixtures/captures/menu.json",
    "outputs": [{ "scene": 0, "file": "...", "seconds": 115.0, "avgFps": 59.8, "worstGapMs": 40, "audio": true }],
    "joined": "...",
    "warnings": ["scene 1: avg 51.2 fps is below 90% of 60"]
  }
  ```
- **Loud failures:** if the auto-accept flag stops working (a prompt or a timeout), or the audio track is missing, exit non-zero with a named error.
- **Replace `web/capture.mjs`** (about 2,000 lines). Delete:
  - the CDP screencast and audio mux;
  - `--width`, `--webm`, `--no-grade`/`capture-grade` and `capture-max`;
  - the `captureConfig.ts` presets;
  - `CinematicCapture.tsx`;
  - `captureSkirmish.tsx`, its route and `web/scenes/capture-skirmish.mjs`.

## Human can see

One command on the Mac turns `fixtures/captures/menu.json` into two scene MP4s, a joined MP4 and a report.

## Verification

- `web/capture.test.mjs` covers the pure stages:
  - argument parsing and refusals;
  - width from height;
  - ffmpeg argv for transcode and join;
  - the report's 90% warning.
- One real short run (an example capture), then the menu capture; ffprobe shows CFR, H.264 plus AAC, 16:9, and the lengths in the report.
- Visual: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on frames pulled around the join crossfade as the last check.

## Delegated

The crossfade length (short, under 1 s), encoder preset/CRF and output naming.

## Stays green

Capture, player, catalog and Rust checks.
