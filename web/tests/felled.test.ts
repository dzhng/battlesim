// A felled tree's stump (`scenery/appearance.ts`): cut from its trunk where
// the tree hinges. How a tree falls and lies is the presentation's
// (`static_world::ground::walk`, tested there). Fixed rules, not the biome's.
import { expect, test } from "vitest";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import { stumpMesh } from "@packages/battle-renderer/src/scenery/appearance";

const CUT = [0.4, 0.3, 0.2];

test("a stump is the trunk's width at its height, its bark's colour, cut at its height", () => {
  // A low-poly trunk as the tree appearances are: a ring at the foot 0.5 m
  // out, the next ring 1 m up 0.3 m out, and leaves that are no bark.
  const v: number[] = [];
  const push = (x: number, y: number, z: number, rgb: number[], foliage: number) =>
    v.push(x, y, z, 0, 0, 1, rgb[0], rgb[1], rgb[2], foliage);
  for (let k = 0; k < 6; k++) {
    const a = (k / 6) * Math.PI * 2;
    push(0.5 * Math.cos(a), 0.5 * Math.sin(a), 0, [0.2, 0.1, 0.05], 0);
    push(0.3 * Math.cos(a), 0.3 * Math.sin(a), 1, [0.2, 0.1, 0.05], 0);
    push(0.2 * Math.cos(a), 0.2 * Math.sin(a), 4, [0.9, 0.9, 0.9], 0);
  }
  push(2.5, 0, 0.2, [0, 0.5, 0], 1);
  const mesh = stumpMesh(Float32Array.from(v), 0.5, CUT);
  let top = 0,
    reach = 0;
  const colours = new Set<string>();
  for (let o = 0; o < mesh.length; o += VERTEX_FLOATS) {
    top = Math.max(top, mesh[o + 2]);
    reach = Math.max(reach, Math.hypot(mesh[o], mesh[o + 1]));
    colours.add([mesh[o + 6], mesh[o + 7], mesh[o + 8]].map((c) => c.toFixed(3)).join(","));
    expect(mesh[o + 9]).toBe(0);
  }
  expect(top).toBeCloseTo(0.5, 9);
  // Half way between the rings' radii, at half the ring's height.
  expect(reach).toBeCloseTo(0.4, 6);
  expect([...colours].sort()).toEqual(["0.200,0.100,0.050", "0.400,0.300,0.200"]);
});
