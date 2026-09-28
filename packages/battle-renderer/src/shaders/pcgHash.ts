// The one integer hash the lattice noises and the grass build draw from:
// PCG's step and output permutation (Jarzynski and Olano, "Hash Functions
// for GPU Rendering", 2020). An integer hash keeps its precision kilometres
// out, where a sin-hash does not.
import { tgpu, d } from "typegpu";

/** The PCG hash of `v`: one LCG step, then the RXS-M-XS permutation. */
export const pcgHash = tgpu.fn(
  [d.u32],
  d.u32,
)(/* wgsl */ `(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}`);
