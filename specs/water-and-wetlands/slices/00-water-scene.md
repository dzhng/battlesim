# 00: The water scene and the frozen reference

**Lane:** render. **Depends on:** nothing. **Visual variable:** none (this slice makes the targets).

## Contract

There is one fixed place to judge water, shared by every visual slice and by
native:

- **Map:** `fixtures/maps/wetlands/`, a fixed test map (decision 31). It has a
  wide river with shallows and a bank, a bridge, a flat lowland patch where
  slice 10 adds the swamp, a dirt track, a small forest, a squad and a tank on
  the bank. It is a test fixture, not game content, and it opens as
  `/labs/wetlands` through `apps/battle-lab/src/fixtures.json`, like
  `/labs/river`.
- **Scene:** `web/scenes/water.mjs`, with three framings: close (~65 m),
  tactical (~260 m) and low angle. Each framing is one `{target, distance,
  pitch, yaw}` record, so native plates and RedGPU captures use the same
  numbers.
- **Water mask:** the simulation-measured mask in `web/scenes/river.mjs`
  (`defineInside`) moves into one shared scene helper, used by both
  `river.mjs` and `water.mjs`. There must not be two copies.
- **Frozen reference:** RedGPU's lake demo
  (`examples/3d/landscape/openWorldIntegration/` and the `lake` examples, at
  tag commit `95aaf9a9ecb3c5ccfcfc824ccad1a6489392cf68`), captured at the same
  pitches and heights. Save the stills to `../assets/` as
  `redgpu-<framing>.jpg`, and record the capture method, browser and commit
  in `../assets/README.md`. The capture script lives in `throwaway/`.
- **Native draw check:** native renders the `wetlands` scene's frames (through
  `web/scenes/native-parity-capture.mjs`), and a check fails if native doesn't
  start or draws no water. Every later slice reruns this check. Today's
  `native/client/tests/pipelines.rs` only builds pipelines.

## The human sees

- `bun run dev`, then `/labs/wetlands`.
- `bun run --cwd web scene -- water`, which writes our current water next to
  the RedGPU stills. Open them with `preview-shots`.

## Verification

1. The scene runs twice and gives identical stills.
2. The native draw check passes on today's water.
3. The existing `river` scene and the `frame/river.png` baseline are
   unchanged after the mask helper moves.
4. Run `screenshot-critique` last, on the three framings only, to check that
   each reads as the shot it claims to be (close, tactical, low). No look is
   judged.
5. Non-blocking checkpoint: show the user our stills beside RedGPU's with
   `preview-shots`. Wait about 5 minutes. If they stay silent, keep the
   framings, record that in the README, and close the shots.

**Stop condition:** if RedGPU can't be captured at a pitch and height
comparable to ours, record that in the README and ask the user before slice
04. Decision 20 then has no trustworthy target.

## Native

This slice only adds the draw check, using today's water. Any gap goes in
[`water-native`](../../water-native/README.md).

## Delegated to the implementer

- exact framings;
- whether the low shot looks along the river or across it;
- the map's dimensions and unit placement;
- how RedGPU is captured (a scripted browser is fine).

## Must stay green

All battle digests and replays, every existing scene and baseline, and native
pipeline validation.

## Feedback that would change this slice

If the user wants a different hero framing, change the shot records. The
slices that follow read them.
