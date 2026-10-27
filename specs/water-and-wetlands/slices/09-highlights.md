# 09: Fresnel and sun highlights

**Lane:** render. **Depends on:** 06 (after 08, if run in order).
**Visual variable:** highlights.

## Contract

- Water's response goes through the shared light owner (`environment.shade`
  and `standardPbr`). There is no second lighting model.
- That means a dielectric Fresnel term (F0 about 0.02) and layered sun lobes at
  rising roughness, plus sun in-scatter, if it reads (RedGPU
  `lakeFragment.wgsl:503-609`).
- **Screen-space reflections call (decision 15):** judge on the low framing
  whether they would earn their cost, and record the call in the README. No
  SSR work happens in this slice.

## Verification

1. Run `compare-screenshots` on the water mask at the low and tactical
   framings, against slice 00's stills.
2. Run `screenshot-critique` last.
3. Non-blocking checkpoint: show the full water look, all three framings beside
   RedGPU, with `preview-shots`. This is the first whole-look read for
   decision 20. Wait about 5 minutes. If the user stays silent, decide on the
   evidence, record it, close the shots and continue.

## Native

Free (shared shader and light).

## Delegated to the implementer

The lobe weights and roughness steps.

## Must stay green

Digests, replays, and slices 04 to 08's crops away from highlights.
