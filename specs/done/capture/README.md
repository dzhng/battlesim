# Capture: scripted battle videos from a JSON file

Landed 2026-10-17 on PR #2 (`cursor/zoom-fade-info-cards-4b65`). An agent writes a capture file that says which saved battles to film and how the camera moves, and one command turns it into MP4s for social media. Nobody drives the tool by hand and it has no UI. The main menu's backdrop is itself a capture, played live by the same scene player, so the menu and the videos are one system: a fix to one is a fix to both.

How to write a capture, the format, the flags, the outputs and the named failures live in the [capture tool's README](../../../apps/battle-lab/src/capture/README.md). This record holds why it has that shape and what must stay true.

## Why it works this way

**Least complexity decides every trade-off.** The tool's only users are agents making marketing clips. Reproducible means a viewer can't tell two runs apart, not bit-exact: a few frames or a few milliseconds of audio drift are acceptable. That ruled out the machinery the old capture route carried: a deterministic mode with frame stepping, offline audio rendering, flash/click calibration with drift correction, a CDP screencast, presets in TypeScript, `CinematicCapture` and a separate skirmish capture route. All of it was deleted when tab capture replaced it (about 5,000 lines in one commit, 5d15f3367).

**Real-time tab capture records what the player sees.** Chromium records its own tab, cropped to the stage element, in wall-clock time: the WebGPU canvas, the DOM info cards, the renderer's grade, the dip layer and every sound, music included. Nothing is re-rendered or re-mixed for the film, so the film cannot drift from the game. The cost is that a slow machine records a choppier video. It doesn't record a slower one, because time is the wall clock, and the report says when a scene fell short. Audio lagged video by about 40 ms in the spike, which was accepted.

**Two files, two owners.** An encounter (`fixtures/maps/<map>/encounters/`) says what happens: units, orders, events and stat overrides. A capture (`fixtures/captures/`) says how it is filmed: scenes and camera only. Tuning a battle for a video is encounter work, so the capture format has no way to carry units, orders or stats.

**Only `capture` maps are filmable.** The map category `menu` became `capture`, with its unit set and `capture_` unit prefix. Test maps stay as their tests need them; capture maps change as often as a film does. To film a test map's ground, copy it into a new capture map. There is no allow-list.

## Invariants

- **One player.** [`ScenePlayer`](../../../apps/battle-lab/src/capture/ScenePlayer.tsx) owns warm-up, elapsed time, the veil, the stage and scene completion, and drives shot sampling and the follow subject through the pilot. The menu ([`MenuBackdrop.tsx`](../../../apps/battle-lab/src/MenuBackdrop.tsx)) mounts it silent under its amber grade; the [capture route](../../../apps/battle-lab/src/routes/capture.tsx) mounts it at full quality, with sound, to record. Neither forks camera logic. The camera pose at a moment comes from the pilot, [`reelPilot.ts`](../../../apps/battle-lab/src/capture/reelPilot.ts), which is pure apart from remembering where its followed unit was.
- **The format has one owner.** [`capture.ts`](../../../apps/battle-lab/src/capture/capture.ts) (`validateCapture`, through `validateReel`) keeps every shot field the reel uses, including `ease`, `logDistance` and `zoomDuration` (the old menu reel loader dropped them), and refuses an unknown field and a non-capture map by name.
- **The camera follows only the filming side's own units (blue).** The client sees only blue's observation, so a red unit can't be followed honestly. The fixture test refuses a red follow; at runtime one would simply never find its unit. A followed unit that is gone leaves the camera where it last stood.
- **A battle's score shuffles by the battle's seed** (`AppAudio.createBattle(options, seed)` in [`appAudio.ts`](../../../packages/battle-audio/src/appAudio.ts)), so a replayed scene plays the same score.
- **The page records; the CLI measures.** The page streams WebM chunks through `captureChunk` and ends with `captureDone({seconds})` or `{error}`. Frame rate, frame gaps and audio presence are measured once, by ffprobe on the recorded file. A second measurement in the page would be a duplicate that can disagree.
- **The film's sound is the tab's mix.** Tab audio is requested stereo with echo cancellation, noise suppression and gain control off ([`tabRecorder.ts`](../../../apps/battle-lab/src/capture/tabRecorder.ts)). A scene without an audio stream fails the run.
- **Failures are loud.** If Chromium stops honouring `--auto-accept-this-tab-capture`, the run fails by name (`TabCaptureNotStarted`) once the recording timeout passes, rather than waiting forever on a prompt ([`recording.mjs`](../../../web/capture/recording.mjs)).
- **Recording needs the user's Mac and a visible (headful) Chromium window.** Tab capture with WebGPU is proven there, never headless or in CI.
- **Never narrow a mechanic to keep the menu reel unchanged.** Re-record the reel's log and approve it after watching.

## Pointers

- The tool: [`apps/battle-lab/src/capture/`](../../../apps/battle-lab/src/capture/) (format, player, pilot, recorder) and its route, [`routes/capture.tsx`](../../../apps/battle-lab/src/routes/capture.tsx).
- The CLI: [`web/capture.mjs`](../../../web/capture.mjs), a thin runner over the pure stages in [`web/capture/`](../../../web/capture/) (`args`, `plan`, `transcode`, `join`, `report`) and the receiver `receiveRecording`, which the CLI and the `capture` browser scene share.
- Captures: [`fixtures/captures/`](../../../fixtures/captures/); [`menu.json`](../../../fixtures/captures/menu.json) is the menu's.
- Map category: `MapCategory` in [`crates/contract/src/maps.rs`](../../../crates/contract/src/maps.rs); the presentation's static-world reader refuses an unknown category by name (`unknown_map_category_is_refused_by_name` in [`static_world/files.rs`](../../../crates/presentation/src/static_world/files.rs)).
- Tests:
  - [`web/tests/capture.test.ts`](../../../web/tests/capture.test.ts): the format, the reel and the pilot;
  - [`web/capture.test.mjs`](../../../web/capture.test.mjs): the CLI's pure stages, including the join and the 90% warning;
  - [`crates/sim/tests/menu_reel.rs`](../../../crates/sim/tests/menu_reel.rs): replays each menu scene against its recorded event log (`crates/sim/tests/fixtures/menu-reel/`), and `every_capture_films_a_capture_map_and_follows_its_own_units`, which refuses a red follow;
  - [`web/scenes/capture.mjs`](../../../web/scenes/capture.mjs): the recording path end to end, a tool the full verify skips.

## Where it diverged from the plan

- **Follow is own units only.** The plan let a shot follow either side. The user said enemy follow was never needed, and the client can't see red's units except through blue's eyes.
- **Scenes join through black, not a dissolve.** The joined MP4 dips through black for 0.5 s (`xfade=transition=fadeblack` with `acrossfade`), as the reel's own cuts do. A dissolve between unrelated shots read as a muddy double exposure.
- **The handoff streams.** The plan had the page count frames (`requestVideoFrameCallback`) and hand back a blob with its stats. The page now streams chunks and reports only `{seconds}`; the frame counter was removed as a duplicate of ffprobe.
- **`--fps` reaches the page.** The CLI passes it to the route as `fps=`, and the tab capture asks for that rate.
- **Found along the way:**
  - the battle score is seeded by the battle seed;
  - the ambience bed ducks under loud nearby shots (803a7b4a6). The battle bed raised earlier on PR #2 (8ae2b8dd0) drowned a near cannon during its booms, failing the native stereo test ([`native/client/tests/audio.rs`](../../../native/client/tests/audio.rs));
  - the exact-numbers parity record was re-blessed for PR #2's light and mix (battle digests unchanged);
  - the `frame` scene's baselines were re-approved for the cooled grade (5492fdbeb): river and geometry against the reference images, foundation (which has none) for unchanged shapes;
  - the rename re-recorded the parity capture, since renaming unit ids reorders the sorted type list, and re-blessed the menu-reel logs after watching both scenes. That also fixed a stale log: PR #2 had retimed the menu's orders without re-recording it.

## Measured

On a quiet machine, the recorder keeps up with the page: the page is the limit. At 1440p the page itself draws about 45 fps, the renderer's limit, and the report warns below 90% of the target rate. At 1080p the top-down scene records about 58 fps. Faster 1440p rendering is outside this work.

The real-page gate (the spike repeated on the real route before anything was deleted) passed on the permission prompt, the crop, the cards and the presence of an audio track (mono at first; see Dead ends). Its fps (57.3 at 1080p top-down, 42.6 at 1440p) was measured on a loaded machine and fell short mostly where the page itself drew slower; the quiet-machine numbers above replaced it.

## Dead ends

- **Deterministic, stepped capture with offline audio.** Bit-exact output was never required, and the machinery to get it (frame stepping, offline rendering, calibration and drift correction) cost about 5,000 lines.
- **Returning the write's result from the exposed chunk function.** Playwright serializes an exposed function's return value back into the page; returning the file write's result sent each chunk back into the page byte by byte, a stall per chunk. The chunk function resolves to nothing.
- **Mono, processed tab audio.** An early gate recorded mono with peaks at 0 dB, until the audio request asked for stereo with voice processing off.
- **Scene flicker at cuts** was reported while the machine was heavily loaded, and didn't reproduce frame by frame in a recording of the cut. Nothing in the player was changed for it.
- **A dissolve between scenes** (see above).

## Visual provenance

- **[The spike](assets/spike/)** ([page](assets/spike/page.html), [runner](assets/spike/run.mjs)) is the proof that the recording path works. A bare WebGPU page with a DOM card, recorded through the same tab-capture calls on the user's Mac on 2026-10-16, showed no permission prompt, an exact crop with the card composited and no browser chrome, and an audio track: 59.9 fps at 1080p and 59.5 fps at 1440p, with audio lagging video by about 40 ms, accepted and not corrected. It drove the choice of tab capture and the fps the real-page gate was held to. Run the runner from `web/` so `playwright` resolves. Its runner returns the whole recording from `page.evaluate`; the shipped CLI streams instead (see Dead ends).
- **[The grade comparisons](assets/grade/)** ([river](assets/grade/river-main-warm-cooled.png), [geometry](assets/grade/geometry-main-warm-cooled.png)) are `frame` scene shots stacked top to bottom: main's grade, PR #2's warm grade, and the cooled grade from 2af0a456c (the warm split tone halved, brightness kept). The cooled grade is the one that shipped, and these are the standard the `frame` baseline re-approval was held to: colour may change, nothing else. They were rendered when the spec was written (026b1df38); `foundation` has no reference image.
