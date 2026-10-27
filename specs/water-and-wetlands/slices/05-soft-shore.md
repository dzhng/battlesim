# 05: The soft shore

**Lane:** render. **Depends on:** 04. **Visual variable:** shoreline softness
and placement.

## Contract

- The water edge fades by optical depth, with a fade distance floored by camera
  distance, so it never aliases (RedGPU `lakeFragment.wgsl:373-375, 630`).
- It replaces the analytic shore bands in `terrainMaterial.ts` (`waterSurface`
  and the shore and bed bands). Those are deleted. The same fade gives swamp
  and puddle edges whatever their shape.
- Water geometry still comes from presentation's water cells.

## Verification

1. The `river.mjs` checks of shore placement against the simulation's water
   edge and of waterline flicker are **expected to fail**, because the shore
   now follows depth on the 4 m triangles. Re-judge each one against what the
   player should see, and rewrite it to test the new mechanic: the edge sits
   where the water depth reaches zero, and doesn't flicker as the camera moves.
   Never loosen a threshold to pass.
2. Run `compare-screenshots` on a bank crop at the close, tactical and low
   framings, against `../assets/redgpu-wading-shore.jpg` and slice 00's
   stills.
3. Run `screenshot-critique` last, on the bank crops.

**Out of scope:** colour against depth (slice 04, frozen), ripples and the bed.

## Native

Free (shared shader).

## Delegated to the implementer

The camera-distance floor constant.

## Must stay green

Digests and replays, and the slice 04 water crop away from the bank.
