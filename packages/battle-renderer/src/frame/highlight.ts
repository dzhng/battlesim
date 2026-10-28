import { tgpu, d, std } from "typegpu";

/** Mesh selection cue, added after lighting in linear RGB. */
export const highlightGlow = tgpu.fn(
  [d.f32],
  d.vec3f,
)((highlight) => {
  "use gpu";
  return std.mul(d.vec3f(0.95, 0.8, 0.2), highlight * 0.7);
});
