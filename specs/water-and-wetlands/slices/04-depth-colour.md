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

## Inherited from slice 02

The water draws multisampled in its own pass, after glass, over the world's
kept samples, with the depth attached read-only. This slice adds the scene
colour copy: `lit` after the world pass, before the water's, single-level
(slice 07 adds mips). The water's shader reads it and the depth. A pipeline
binds at most four groups, and the water already uses four, so both must go
into one of them ([choices](../choices.md), slice 02).

## Native

The shader is shared. Native adds the same copy and binding as the browser.

## Delegated to the implementer

The absorption constants per look row.

## Must stay green

Battle digests and replays, and everything outside the water mask.
