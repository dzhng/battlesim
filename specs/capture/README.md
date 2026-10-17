# Capture: scripted battle videos from a JSON file

Status: slice 01 done 2026-10-17 (rename landed; parity capture re-recorded whole; menu-reel logs re-blessed after watching both scenes).

## Next Agent Prompt

You are resuming the capture rework. Read this README, then the slice you pick up.

- **Next pickup:** [slice 02, capture module](slices/02-capture-module.md). The pure CLI stages of slice 05 and the `frame` re-approval of slice 06 don't depend on 02–04 and can run alongside.
- **Machine:** recording runs headful Chrome on the user's Mac. The spike proved it there. Don't try it headless or in CI.
- **TODO:**
  - [x] 01 Rename the map category, unit set and unit prefix `menu` → `capture`, re-record the parity and menu-reel logs ([slice](slices/01-rename-menu-to-capture.md))
  - [ ] 02 Capture file and the `capture` module ([slice](slices/02-capture-module.md))
  - [ ] 03 One `ScenePlayer` for menu and capture, any-side follow, seeded score ([slice](slices/03-scene-player.md))
  - [ ] 04 Capture route records the stage with tab capture ([slice](slices/04-tab-recorder.md))
  - [ ] 05 CLI writes the MP4s and the report ([slice](slices/05-cli.md))
  - [ ] 06 Menu reel through the new path, approvals, docs, full run, close-spec ([slice](slices/06-closeout.md))
- Update this section (status, pickup, ticks, blockers) before you end every pass.

## Goal

The capture tool exists so an agent can script marketing videos. Nobody uses it by hand, and there's no UI (the user, 2026-10-16). An agent writes a JSON file that says which battles to film and how to move the camera. One command turns it into MP4s. The main-menu backdrop is the first capture and plays through the same player, so menu and videos stay one system.

Least complexity and least technical debt decide every trade-off. Output is 16:9 for social media, 1080p by default and 1440p at most; 4K is out of scope. Reproducible means a viewer can't tell two runs apart. It doesn't mean bit-exact: a few frames or a few milliseconds of audio drift are acceptable.

## Contracts

**Two concepts, one owner each.**

- **Encounter** (exists, unchanged) says what happens. It lives at `fixtures/maps/<map>/encounters/*.json` and holds units, scripted orders and events, and stat `overrides`, read by `EncounterDefinition::on` (`crates/contract/src/scenario.rs`). Tuning units for a video, which the menu does a lot, is encounter work.
- **Capture** (new name for the menu backdrop file) says how it's filmed. It covers camera and scenes only and never touches stats.

```jsonc
// fixtures/captures/<name>.json
{
  "scenes": [
    {
      "map": "market-town",        // a map whose category is "capture"
      "encounter": "menu",         // file stem under that map's encounters/
      "seed": 7,                   // battle seed; also the score's shuffle seed
      "warm_s": 4,                 // simulated before the first frame
      "reel": {
        "fade_s": 0.6,             // dip between shots
        "shots": [ /* every Shot field validateCapture keeps, including follow, ease, logDistance, zoomDuration */ ]
      }
    }
  ]
}
```

- `follow` is an index into the encounter's `units`, either side. It replaces today's blue-only lookup (`MenuBackdrop.tsx:350`).
- `validateCapture` keeps every shot field. Today `menuReel.ts:61-66` drops `ease`, `logDistance` and `zoomDuration`.
- It refuses non-capture maps and unknown fields, by name.

**Map category `capture`** replaces `menu`, and the unit catalog set and the `menu_` unit prefix become `capture` and `capture_`. Only capture maps are filmable. Test maps never are: copy one into a capture map instead (the user: "a test map should stay a test map"). Capture maps change often; test maps stay static.

**One player.** `ScenePlayer`, extracted from `apps/battle-lab/src/MenuBackdrop.tsx`, owns:
- the warm-up tick and elapsed time;
- shot sampling (camera pose from the reel at time t);
- the follow subject and the fade veil;
- the stage element and scene completion.

The menu mounts it live under its grey film look. The capture route mounts it to record. Neither forks the camera logic.

**Recording** is Chrome tab capture in real time:
1. The page calls `getDisplayMedia({preferCurrentTab:true, audio:true, video:{frameRate}})`, auto-accepted by the launch flag `--auto-accept-this-tab-capture`.
2. It crops with `CropTarget.fromElement(stage)` + `track.cropTo`.
3. `MediaRecorder` writes VP9+Opus WebM.
4. The CLI transcodes that to constant-frame-rate H.264/AAC MP4.

It records what the player sees: WebGPU, DOM info cards, the grade, the dip layer, and all sound including music.

**CLI.** `bun run --cwd web capture <file.json>` takes:

| Flag | Default |
|---|---|
| `--height=1080\|1440` | 1080; width is height × 16/9 |
| `--fps` | 60 |
| `--no-cards` | cards on |
| `--output` | |

The grade is always on.

**Outputs.**
- One MP4 per scene.
- One joined MP4 with a short `xfade`/`acrossfade` crossfade, when there's more than one scene.
- `<name>.report.json`: the capture file, each output, length, average and worst fps, whether audio is present, and warnings.

**Slow machines degrade gracefully.** Below target fps the video gets choppier but never drifts out of sync or speed, because the recording is wall-clock real time. The report warns when average fps is below 90% of target.

## Slice graph

```
01 rename ──► 02 capture module ──► 03 ScenePlayer ──► 04 tab recorder ──► 05 CLI ──► 06 closeout
```

The order is linear because each slice consumes the previous slice's names. Old code goes in the slice that replaces it (04 removes the route's deterministic paths, 05 removes the CLI's screencast paths), so nothing waits for a cleanup slice.

| Slice | Human sees | Gate |
|---|---|---|
| 01 | Menu still plays both scenes; catalogue lists 2 capture and 18 test maps | rename tests, parity + menu-reel re-record after watching |
| 02 | Menu loads from `fixtures/captures/menu.json` | `capture` unit tests |
| 03 | Menu unchanged; a red unit can be followed | menu first-cut probe, `frame` scene, critique |
| 04 | `/labs/capture` records a WebM with cards and sound | `capture` scene, ffprobe, critique |
| 05 | One command → MP4s + report | CLI stage tests, a real run, ffprobe |
| 06 | Menu reel re-recorded through the new path | approvals, docs, full run |

## What is deleted

Hard cutover: no compatibility aliases, migrations or shims (user, 2026-10-16). Each of these goes in the slice named next to it:

- **Slice 04:**
  - CDP screencast;
  - flash/click calibration and drift/atempo correction;
  - deterministic mode and `stepFrame`;
  - `RecordingSink`/`renderOffline` and the offline WAV;
  - `createCaptureTap`;
  - `captureCalib.mjs`.
- **Slice 05:**
  - `captureSkirmish.tsx`, its route and `web/scenes/capture-skirmish.mjs`;
  - the `captureConfig.ts` presets, which become example capture files `fixtures/captures/{default,top-down,ground-level,orbit}.json`;
  - `CinematicCapture.tsx`;
  - the `--no-grade`/`capture-grade`, `--webm`, `capture-max` and `--width` options.

## Firewalls

- The capture file never carries stats, units or orders; those belong to the encounter.
- No test map is filmable, and there's no allow-list escape hatch.
- The `menu` audio screen/mood and the `.menu-backdrop` CSS stay. They're about the menu screen, not the map category.
- Leave `specs/done/` alone, even where it says `menu`.
- Keep the PR's audio retune (the user's own feedback) and the grade as pushed in 2af0a456c ([before/after](assets/grade/)).
- Don't build UI for the tool, and don't add bit-exact determinism, frame stepping or offline audio.
- `graphicsTest.tsx` also reads reels. It's out of scope; adapt its imports only.
- Never narrow a game mechanic to keep the reel unchanged. Re-record it and approve it instead.

## Proven behaviour (spike, frozen in [assets/spike](assets/spike/))

The [spike page](assets/spike/page.html) and [runner](assets/spike/run.mjs) ran on the user's Mac on 2026-10-16. Run the runner from `web/` so `playwright` resolves.

| Behaviour | Evidence | Owner slice |
|---|---|---|
| No permission prompt with `--auto-accept-this-tab-capture` | log "cropped", 1 audio track | 04 |
| WebGPU canvas and DOM card composite, exact crop, no browser chrome | frame grab | 04 |
| 1080p | 597 frames, 59.9 fps, max gap 35 ms | 04, 05 |
| 1440p | 59.5 fps, max gap 68 ms | 05 |
| Audio | lags video 15–62 ms (about 40 ms average); accepted, not corrected | 05 |

Slice 04's first step repeats this on the real `/labs/capture` page before anything is deleted.

## Known unknowns

- **Auto-accept flag.** If a Chrome update breaks `--auto-accept-this-tab-capture`, the CLI must fail loudly with a named error, never hang on a prompt.
- **Following a fogged unit.** The player follows through the unfogged (x-ray) observation. If a followed red unit dies, the camera holds its last position.
- **Name collision.** "Capture" already means the game's capture points (`specs/battle-foundation/slices/21-capture-economy.md`) and the native parity captures (`fixtures/parity/capture/`, `web/scenes/native-parity-capture.mjs`). The user chose "capture" and "scene" for this feature. Renaming the parity collision is a follow-up the user decides; it isn't in this spec.

## Alternatives considered

Three drafts were weighed on 2026-10-16:
- **Fewest slices (2):** rename plus model in one slice, then route plus CLI.
- **Risk-first (6):** gate on the real page before any rename.
- **Seam quality (9):** score seed, pilot and CLI stages as separate slices.

The ladder takes risk-first's early real-page gate, folded into slice 04's first step. It takes seam quality's seams: a pure `reelPilot`, the CLI's pure stages, and the parity re-record in the rename. It merges the small slices so each one still leaves a runnable artifact.
