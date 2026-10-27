# 06: The water surface: wind-driven normals

**Lane:** render. **Depends on:** 01 and 05.
**Visual variable:** surface detail and motion.

## Contract

- **The normal texture is generated once at load** by an exported bake
  pipeline, next to `bake.wgsl` (decision 16), with one lifetime owner. No
  image files ship, and RedGPU's unlicensed maps are not used.
- **The shader samples it at three scales:** primary, a rotated macro layer and
  a rotated detail layer. They blend with RNM, each scrolling along the
  slice 01 `Wind` at its own rate, and fade toward flat with distance
  (RedGPU `lakeFragment.wgsl:280-346`).
- **Deleted:** `waterNormal`'s procedural ripple octaves.
- **Vertex waves:** judged at the low framing. They are dropped if they don't
  read.

## Verification

1. Run `compare-screenshots` on the water mask at all three framings. Targets:
   `../assets/redgpu-overhead-shadows.jpg` and slice 00's stills. Include a
   motion pair: two frames 0.5 s apart on the presentation clock.
2. Check that pausing freezes the surface, because the clock stands still.
3. Probe the bake's cost and the per-frame cost.
4. Run `screenshot-critique` last, on the still and the motion pair.

**Out of scope:** highlights (slice 09). The surface stays on the existing
`environment.shade`.

## Native

Port the one-time bake. If that's hard, native draws a flat normal and a
[`water-native`](../../water-native/README.md) item records it.

## Delegated to the implementer

- texture resolution and the generator;
- the scroll rates per layer;
- whether to keep vertex waves.

## Must stay green

Digests, replays, and slices 04 and 05's crops when motion is frozen.
