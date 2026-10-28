# 18: Closeout

**Depends on:** every other slice.

## Contract

- **Every tier and both clients start and draw.** Low drops refraction,
  caustics and wakes and keeps the depth colour.
- **Every native gap is listed** in [`water-native`](../../water-native/README.md),
  each with what native draws instead.
- **Done** is decision 20's side-by-side read: at our camera, next to RedGPU,
  ours reads as good.
- **The far water keeps its surface.** Slice 09's checkpoint found the
  tactical river an untextured band: its normals fade flat at 260 m. Keep
  the slope variance a pixel averages away as roughness, so it keeps a
  varying sheen ([README](../README.md#still-open)).

## Verification

1. Run everything once (`AGENTS.md`): `bun run check`, `bun run verify`, the
   full scene suite, native parity, and the pipeline and naga checks.
2. Benchmark at the three framings on High and Low.
3. Run `compare-screenshots` on the whole frame at all three framings beside
   RedGPU. This is the only whole-frame judgement in the spec.
4. Run `screenshot-critique` last.
5. Show the user the final shots with `preview-shots`, then run `review` and
   `close-spec`.
