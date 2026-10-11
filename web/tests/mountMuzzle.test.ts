// The browser's muzzle model matches the simulation's (`weapons::placed_muzzle`):
// a roof gun on an off-centre turret swings round that turret's ring, not
// round the hull's middle.
import { expect, test } from "vitest";
import { vec3 } from "math";
import { mountMuzzles, muzzleOffset } from "@packages/scene-assets/src/mountMuzzle";

const [cannon, hmg] = mountMuzzles([
  { id: "cannon", on: null, pivot_m: [-1, 0, 1.45], muzzle_m: [5.9, 0, 0.55] },
  { id: "HMG", on: "cannon", pivot_m: [-1.25, -0.58, 2.35], muzzle_m: [1.43, 0, 0.32] },
]);

test("a roof gun on an off-centre turret stays over the turret as it turns", () => {
  const at = vec3.create();
  for (const carried of [0, Math.PI / 2, Math.PI, -Math.PI / 2]) {
    muzzleOffset(at, hmg!, 0, carried, carried);
    // Its pivot keeps its distance from the turret's ring whatever the bearing.
    const pivot = vec3.sub(vec3.create(), at, [
      1.43 * Math.cos(carried),
      1.43 * Math.sin(carried),
      0.32,
    ]);
    const fromRing = Math.hypot(pivot[0] - -1, pivot[1] - 0);
    expect(fromRing).toBeCloseTo(Math.hypot(0.25, 0.58), 9);
  }
});

test("a turret pointing over the bow turns with the hull as one", () => {
  // Hull and turret both heading 90°: the whole mount row turns as the hull does.
  const at = vec3.create();
  muzzleOffset(at, hmg!, Math.PI / 2, Math.PI / 2, Math.PI / 2);
  expect([...at].map((x) => Number(x.toFixed(9)))).toEqual([0.58, 0.18, 2.67]);
});
