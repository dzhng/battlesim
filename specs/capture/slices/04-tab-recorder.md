# 04 — Capture route records the stage

## Contract unlocked

`/labs/capture?capture=<name>&scene=<i>&cards=0|1&record=1` plays one scene through `ScenePlayer` and records the stage as VP9+Opus WebM. The recording includes the WebGPU canvas, high-air canvas, DOM cards, dip layer, grade and all sound. It exposes the result to the CLI.

## Seam

- **`tabRecorder.ts`** exposes `recordTab(stage, {fps, until}) → {blob, frames, worstGapMs}`, built from the [spike page](../assets/spike/page.html):
  - `getDisplayMedia({preferCurrentTab:true, audio:true, video:{frameRate}})`;
  - `CropTarget.fromElement(stage)` + `track.cropTo`;
  - `MediaRecorder`.

  It counts frames with `requestVideoFrameCallback` on the track.
- **The route** pins the graphics tier high, turns reduced motion off and skips graphics detection, whatever is saved in settings (see `MenuBackdrop.tsx:128-174,224`). It starts recording only when the stage is ready and stops on `ScenePlayer`'s `onEnd`.
- **Rewrite** the existing capture route (about 1,000 lines) down to this. Delete:
  - deterministic refs and `stepFrame`;
  - `RecordingSink`, `renderOffline` and the offline WAV;
  - `createCaptureTap` and `captureCalib.mjs`.

  Remove `hearAt`/`updateWithListener` if nothing else uses them.

## First step: real-page gate

Before deleting anything, run the new recorder on the real route. Use headful Chromium on the Mac with:
- `--auto-accept-this-tab-capture`;
- `--autoplay-policy=no-user-gesture-required`;
- `ignoreDefaultArgs:['--mute-audio']`.

Record 10 s at 1080p and at 1440p, then check:
- no prompt;
- the crop is exact, with cards visible;
- an audio track is present;
- fps is within 10% of the spike (59.9 / 59.5).

If it fails, stop and update the README; don't work around it.

## Human can see

A WebM of a menu scene with cards and sound.

## Verification

- `web/scenes/capture.mjs` is rewritten: it drives the route, awaits the recorder, and asserts that a video and an audio stream are present (ffprobe).
- Visual: use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on a frame from the WebM against the live menu frame at the same reel time. Slice variable: crop and composition (cards, grade, no chrome). The menu's grey film look is an expected difference. Then run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the frame as the last check.

## Delegated

Bitrate, the WebM chunk interval, and how the route hands the blob to the CLI (exposed function or download).

## Stays green

The menu route, `ScenePlayer` tests and the `frame` scene.
