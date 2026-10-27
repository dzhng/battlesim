# 16: The wave field

**Lane:** wet. **Depends on:** 06 and 15. **Visual variable:** wake rings and
V-wakes.

## Contract

- **Clock:** `presentation::wave::WaveClock::advance(clock, camera)` returns
  `WaveStep { steps, epoch, origin_texel, shift }`.
  - `steps` is a fixed-rate step count, capped, so a jump drops the excess.
  - `epoch` changes on a reset, a side switch or a load (decision 24).
  - The accumulator is kept in f64.
- **Field:** the renderer runs a ping-pong of `rgba16float` render targets in
  fragment passes. It uses no storage textures, so native's schema is
  unchanged.
  - The wave equation is RedGPU's `lakeWaveSimulation.wgsl`: a 9-point
    Laplacian, damping and an absorbing border.
  - The field clears when the epoch changes, shifts by whole texels as the
    camera moves, and splats each impulse at its step.
  - Ripples blend into slice 06's normals with RNM. The renderer never chooses
    who makes a wake.
- **Area:** follows the camera, sized to the tactical view by the cost probe
  below. Wakes fade out at far zoom (decision 14). Off on Low (decision 18).
- **Captures:** warm up a fixed number of steps from a reset, so shots repeat.

## Seam tests

- the field steps the same number of times at any frame rate;
- a reset, a side switch or a load changes the epoch;
- the area moves in whole texels.

## Verification

1. **Cost probe:** step cost for each candidate area size at the tactical
   framing. Record the choice.
2. Run `compare-screenshots` on a wake crop at the close and tactical framings,
   against `../assets/redgpu-wake-rings.jpg` and slice 00's stills, after a
   fixed warm-up.
3. A frame after a side switch shows no waves.
4. Run `screenshot-critique` last, on the wake crop.
5. Non-blocking checkpoint: show it with `preview-shots`. Wait about 5 minutes.
   If the user stays silent, decide, record the decision and continue.

## Native

Deferred to [`water-native`](../../water-native/README.md). Native draws no
wakes, and the wave state stays out of plate parity.

## Delegated to the implementer

- the texel count;
- the damping;
- the splat shape.

## Must stay green

Digests, replays, and slice 06's surface crops when there are no wakes.
