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
- **The wind every pass reads is the camera's** (`Camera.wind`), not the
  environment uniform's: a sun cascade's caster binds only its camera, so
  the one `Wind` record moved there, every cascade camera runs on the
  frame's clock and wind, and the environment's copy is deleted
  ([choice 1](../choices.md#slice-17-tree-sway)). No bind-group layout
  changed; the camera's struct grew, which native uploads from the
  presentation's records as they are.

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
