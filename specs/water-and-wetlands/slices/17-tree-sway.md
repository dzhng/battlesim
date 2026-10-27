# 17: Tree sway

**Lane:** render, parallel any time after slice 01. **Depends on:** 01.
**Visual variable:** canopy motion.

## Contract

- **One WGSL function**, `sway(world, height, wind)`, is shared by the
  scenery prepass, colour and shadow-caster pipelines.
  - Trunks bend, and leaves flutter (RedGPU `foliageInstanced.wgsl:38-110`).
  - Casters bend the trunk only.
  - The prepass and colour pipelines must call the identical function, or the
    equal-depth test leaves holes.
- Sway is look only. Felled trees lying on the ground don't sway.

## Verification

1. Run `compare-screenshots` on a forest crop at the close and tactical
   framings: two frames 0.5 s apart, against today's still forest.
2. Check a crop for holes along canopy edges.
3. Check the menu reel's liked shots read the same apart from the motion.
4. Run `screenshot-critique` last, on the motion pair.

## Native

Must be ported in this slice if a bind-group layout changes, because
`by_layout` panics at draw time. Otherwise native gets it free.

## Delegated to the implementer

Amplitude, phase and the height weighting.

## Must stay green

Digests, replays, and shadow shots with the wind frozen.
