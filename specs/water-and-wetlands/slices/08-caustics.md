# 08: Caustics

**Lane:** render. **Depends on:** 07. **Visual variable:** the caustic pattern
on the bed.

## Contract

Procedural caustics are computed in the water shader:

- two samples of the normal texture, each distorting the other's coordinates,
  with crests where they agree;
- projected onto the bed reconstructed from depth, along the view and the sun;
- faded with depth, and masked off steep walls (RedGPU
  `lakeFragment.wgsl:435-501`).

They are gated by `waterDetail` and are off on Low.

## Verification

1. Run `compare-screenshots` on the shallows at the close and overhead
   framings, against `../assets/redgpu-overhead-shadows.jpg`, with refraction
   frozen at slice 07's accepted output.
2. Perf: the added taps.
3. Run `screenshot-critique` last, on the shallows crop.

**Out of scope:** highlights.

## Native

Free (shared shader).

## Delegated to the implementer

The tap count and the depth fade.

## Must stay green

Digests, replays, and Low shots.
