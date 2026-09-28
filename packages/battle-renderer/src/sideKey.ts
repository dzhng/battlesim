// One number per (side, id), so the two sides' unit and soldier ids never
// collide in one map: the `even` side's ids even, the other side's odd. The
// effects, the sound and the drawn muzzles all key this way.
import type { Side } from "@packages/scene-assets/src/schema";

/** The key of `side`'s `id`, with `even`'s ids even. */
export function sideKey(id: number, side: Side, even: Side): number {
  return id * 2 + (side === even ? 0 : 1);
}

/** The id and side a `sideKey` names. */
export function fromSideKey(key: number, even: Side): { id: number; side: Side } {
  return {
    id: Math.floor(key / 2),
    side: key % 2 === 0 ? even : even === "blue" ? "red" : "blue",
  };
}
