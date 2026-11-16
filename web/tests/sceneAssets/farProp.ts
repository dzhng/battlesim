// Synthetic sources for the far mesh: a static prop that may name one, and
// a copy of any source with one added. Generated in code, like synthetic.ts.

import type { Vec3 } from "math";
import type { GltfJson } from "@packages/scene-assets/src/glb.ts";
import { GltfBuilder } from "./gltfBuilder";
import { gBox, withJson } from "./synthetic";

/** The far mesh's slab (`farPropGlb`): squat, at the walls' foot, so its
 *  bounds tell it apart from every tier. */
export const FAR_SLAB: [Vec3, Vec3] = [
  [-4, -3, 0],
  [4, 3, 2],
];

/**
 * A 10 × 8 m static prop, `height` tall: tier t draws 4 − t wall boxes, and
 * an unsuffixed lid is in every tier from 0 to 3, so tier 3 is two boxes (24
 * triangles). Its far mesh (`_LOD4`) is `far` boxes of `FAR_SLAB`; 0 names a
 * `_LOD4` mesh with no geometry, and leaving it out names none.
 */
export function farPropGlb(far?: number, height = 6): Uint8Array {
  const b = new GltfBuilder();
  const walls = gBox(b, [-5, -4, 0], [5, 4, height]);
  const parts = [0, 1, 2, 3].flatMap((tier) =>
    Array.from({ length: 4 - tier }, (_, i) =>
      b.node({ name: `walls${i}_LOD${tier}`, mesh: walls }),
    ),
  );
  parts.push(b.node({ name: "lid", mesh: gBox(b, [-1, -1, height - 0.5], [1, 1, height]) }));
  if (far !== undefined) {
    const slab = gBox(b, ...FAR_SLAB);
    for (let i = 0; i < far; i++) parts.push(b.node({ name: `slab${i}_LOD4`, mesh: slab }));
    if (far === 0) {
      b.json.meshes.push({ primitives: [] });
      parts.push(b.node({ name: "slab_LOD4", mesh: b.json.meshes.length - 1 }));
    }
  }
  b.roots(b.node({ name: "prop", children: parts }));
  return b.glb();
}

/** `bytes` with a far mesh: a copy of its first `_LOD3` mesh node (or its
 *  first mesh node, where none names a tier) named `_LOD4`, beside it under
 *  the same parent. */
export function withFarCopy(bytes: Uint8Array): Uint8Array {
  return withJson(bytes, (json) => {
    const nodes = json.nodes as GltfJson[];
    const meshNode = (n: GltfJson, tiered: boolean) =>
      n.mesh !== undefined && (!tiered || /_LOD3$/.test(n.name));
    const tiered = nodes.findIndex((n) => meshNode(n, true));
    const at = tiered >= 0 ? tiered : nodes.findIndex((n) => meshNode(n, false));
    const { children: _, ...copy } = nodes[at];
    nodes.push({ ...copy, name: `${copy.name.replace(/_LOD3$/, "")}_LOD4` });
    const parent = nodes.find((n) => n.children?.includes(at));
    (parent ? parent.children : json.scenes[0].nodes).push(nodes.length - 1);
  });
}

/** A 10 × 8 × 6 m block whose tiers 0 to 2 are the whole box and whose tier
 *  3 is only two of its faces (4 triangles): smaller than any far box. */
export function tinyPropGlb(): Uint8Array {
  const b = new GltfBuilder();
  const whole = gBox(b, [-5, -4, 0], [5, 4, 6]);
  const two = gBox(b, [-5, -4, 0], [5, 4, 6]);
  b.json.accessors[b.json.meshes[two].primitives[0].indices].count = 12;
  const parts = [0, 1, 2].map((t) => b.node({ name: `block_LOD${t}`, mesh: whole }));
  parts.push(b.node({ name: "block_LOD3", mesh: two }));
  b.roots(b.node({ name: "prop", children: parts }));
  return b.glb();
}

/** The two-tone block's colours: pale walls, a dark roof. */
export const TWO_TONE = { walls: [0.8, 0.8, 0.78, 1], roof: [0.05, 0.05, 0.06, 1] } as const;

/** A 10 × 8 m block of pale walls 6 m tall under a dark roof slab 0.2 m
 *  thick, the same in every tier: from above it shows its roof, from every
 *  side its walls. */
export function twoToneGlb(): Uint8Array {
  const b = new GltfBuilder();
  b.json.materials[0].pbrMetallicRoughness.baseColorFactor = [...TWO_TONE.walls];
  b.json.materials.push({
    name: "roof",
    pbrMetallicRoughness: {
      baseColorFactor: [...TWO_TONE.roof],
      metallicFactor: 0,
      roughnessFactor: 0.9,
    },
  });
  const walls = gBox(b, [-5, -4, 0], [5, 4, 6]);
  const roof = gBox(b, [-5, -4, 6], [5, 4, 6.2]);
  b.json.meshes[roof].primitives[0].material = 1;
  const parts = [0, 1, 2, 3].flatMap((t) => [
    b.node({ name: `walls_LOD${t}`, mesh: walls }),
    b.node({ name: `roof_LOD${t}`, mesh: roof }),
  ]);
  b.roots(b.node({ name: "prop", children: parts }));
  return b.glb();
}

/** Two posts 0.2 m square and 3 m tall at opposite corners of a 4 × 4 m
 *  plan, at (−2, −2) and (2, 2): a prop that covers little of its bounds. */
export function postsGlb(): Uint8Array {
  const b = new GltfBuilder();
  const posts = [-2, 2].map((c) => gBox(b, [c - 0.1, c - 0.1, 0], [c + 0.1, c + 0.1, 3]));
  const parts = [0, 1, 2, 3].flatMap((t) =>
    posts.map((mesh, k) => b.node({ name: `post${k}_LOD${t}`, mesh })),
  );
  b.roots(b.node({ name: "prop", children: parts }));
  return b.glb();
}
