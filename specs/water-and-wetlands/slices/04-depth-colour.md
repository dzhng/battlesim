# 04: Depth colour

**Lane:** render. **Depends on:** 03.
**Visual variable:** colour against depth, in the two river looks.

## Contract

- Water colour comes from the optical depth between the water surface and the
  scene depth. It is per-channel absorption from the look's shallow colour to
  its deep colour, over the scene colour copy. RedGPU's formula
  (`lakeFragment.wgsl:415-433`) is the reference.
- **Paint ownership moves to the water layer** (README, sharp edges). The bed
  no longer paints under water, so ground marks show once.
- Water's fog coverage is re-judged now that water is opaque over its bed.
- The existing shore and normals stay as they are. They are later slices.

## Verification

1. Run `compare-screenshots` at the close and tactical framings, on the water
   mask only. Targets: `../assets/redgpu-depth-colour.jpg` and slice 00's
   frozen stills. Telemetry: the luminance and hue of shallow and deep bands.
   Verdict: less wrong.
2. Order lines over water appear once and don't double.
3. Re-record `web/scenes/baselines/frame/river.png`. It is taken on Low, which
   keeps depth colour (decision 18).
4. Run `screenshot-critique` last, on the water crop, for each river look.
5. Non-blocking checkpoint: show both looks with `preview-shots`. Wait about 5
   minutes. If the user stays silent, decide on the evidence, record the
   decision, close the shots and continue.

**Out of scope, even if visibly wrong:** shore softness (slice 05), ripples
(06), the bed's sharpness and offset (07), caustics (08) and highlights (09).

## Native

Gets this for free. The shader is shared, and the copy came with slice 02.

## Delegated to the implementer

The absorption constants per look row.

## Must stay green

Battle digests and replays, and everything outside the water mask.
