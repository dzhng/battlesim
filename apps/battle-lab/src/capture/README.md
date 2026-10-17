# Capture

Scripted battle videos for marketing, filmed from a JSON file. An agent writes
the file; nobody drives this by hand, and there is no UI. The design and its
reasons live in [the capture spec](../../../../specs/capture/README.md).

## Two files, two owners

- An **encounter** (`fixtures/maps/<map>/encounters/*.json`) says what
  happens: units, orders, events and stat overrides.
- A **capture** (`fixtures/captures/<name>.json`) says how it is filmed: a list
  of scenes, each a capture map, an encounter, a seed, a warm-up and a reel of
  camera shots. It never carries stats or units. `capture.ts` validates it;
  only maps whose category is `capture` are filmable.

The main menu's backdrop is the `menu` capture, played live by the same
`ScenePlayer` the capture route records.

## Filming

```bash
bun run --cwd web capture fixtures/captures/<name>.json
```

The command's usage text (`web/capture/args.mjs`) owns the flags. It plays each
scene on `/labs/capture` in headful Chromium on the Mac, which records the
stage, its cards and every sound with tab capture (`tabRecorder.ts`) in real
time, then writes to `throwaway/capture/` by default:

- one constant-rate H.264/AAC MP4 per scene;
- one joined MP4 with a short crossfade, when there are several scenes;
- `<name>.report.json`: each output's length, average fps, worst frame gap,
  audio, and warnings.

A slow machine records choppier, never slower or out of sync; the report warns
below 90% of the target fps. A capture prompt, a hang or a missing audio track
fails the command by name.
