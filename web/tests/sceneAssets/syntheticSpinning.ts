// Synthetic stand-ins with parts that spin: a skid helicopter, a helicopter
// with its rotors, and a cart on its wheels (the articulation and rubber
// checks' fixtures), built with `synthetic.ts`'s glTF builder.
import type { Vec3 } from "math";
import { GltfBuilder } from "./gltfBuilder";
import { LODS, g3, gBox, qx, tieredPart } from "./synthetic";

/** A skid helicopter's stand-in: a fuselage block resting on two skid rails
 *  of painted metal; no wheels. */
export function skidGlb(): Uint8Array {
  const b = new GltfBuilder();
  const body = tieredPart(b, "fuselage", gBox(b, [-4, -0.8, 0.3], [4, 0.8, 2.2]), LODS.length);
  const rails = ["L", "R"].map((s, k) => {
    const y = k === 0 ? 1 : -1;
    return tieredPart(b, `rail_${s}`, gBox(b, [-2, y - 0.05, 0], [2, y + 0.05, 0.1]), LODS.length);
  });
  b.roots(b.node({ name: "heli", children: [body, ...rails] }));
  return b.glb();
}

/** The synthetic helicopter's main rotor and tail rotor radii (blade tip
 *  from the hub, measured as the renderer does: the box corner's reach). */
export const HELI_ROTORS = { main: Math.hypot(5, 0.15), tail: Math.hypot(0.7, 0.08) };

/** A helicopter in engine space, fitting the synthetic `heli` type's hull
 *  [4, 0.8, 1.2]: a fuselage block on skid rails, a mast to the rotor head
 *  at z 2.4 (its top 0.2 m the rotor's own), and its rotors, each a `rotor_*` node turning about its local
 *  Z: the main rotor's blade 10 m tip to tip along +X at rest (far past the
 *  hull), the tail rotor's 1.4 m on the boom's left, turned on its side. */
export function heliGlb(): Uint8Array {
  const b = new GltfBuilder();
  // Boxes per tier, 12 triangles each: 6, 4, 3, 2.
  const part = (name: string, min: Vec3, max: Vec3, tiers = LODS.length) =>
    tieredPart(b, name, gBox(b, min, max), tiers);
  const main = b.node({
    name: "rotor_main",
    t: g3([0.5, 0, 2.4]),
    children: [
      part("blade_main_0", [-5, -0.15, -0.03], [5, 0.15, 0.03]),
      // Its mast, down from the hub to the hull's, as an exporter builds one.
      part("rotor_main_mast", [-0.08, -0.08, -0.2], [0.08, 0.08, 0], 2),
    ],
  });
  const tail = b.node({
    name: "rotor_tail",
    t: g3([-3.8, 0.9, 1.6]),
    r: qx(-90),
    children: [part("blade_tail_0", [-0.7, -0.08, -0.02], [0.7, 0.08, 0.02], 3)],
  });
  const hull = b.node({
    name: "hull",
    children: [
      part("fuselage", [-4, -0.8, 0.3], [4, 0.8, 2.2]),
      part("mast", [0.4, -0.1, 2.2], [0.6, 0.1, 2.4], 2),
      ...["L", "R"].map((s, k) => {
        const y = k === 0 ? 0.75 : -0.75;
        return part(`rail_${s}`, [-2, y - 0.05, 0], [2, y + 0.05, 0.1], 1);
      }),
      main,
      tail,
    ],
  });
  b.roots(b.node({ name: "heli", children: [hull] }));
  return b.glb();
}

/** A cart on four rubber tyres (material role `rubber`), each tyre under
 *  its own `wheel_*` node, or, with `spinning` false, fixed to the body. */
export function cartGlb(spinning = true): Uint8Array {
  const b = new GltfBuilder();
  b.json.materials.push({
    name: "rubber",
    pbrMetallicRoughness: {
      baseColorFactor: [0.02, 0.02, 0.02, 1],
      metallicFactor: 0,
      roughnessFactor: 0.9,
    },
    extras: { role: "rubber" },
  });
  const body = tieredPart(b, "body", gBox(b, [-2, -1, 0.5], [2, 1, 1.5]), LODS.length);
  const tyres = [
    ["FL", 1.5, 1],
    ["FR", 1.5, -1],
    ["RL", -1.5, 1],
    ["RR", -1.5, -1],
  ].map(([name, x, y]) => {
    const mesh = gBox(b, [-0.4, -0.15, -0.4], [0.4, 0.15, 0.4]);
    b.json.meshes[mesh].primitives[0].material = 1;
    const tyre = tieredPart(b, `tyre_${name}`, mesh, LODS.length);
    const at = g3([x as number, y as number, 0.4]);
    return spinning
      ? b.node({ name: `wheel_${name}`, t: at, children: [tyre] })
      : b.node({ name: `hub_${name}`, t: at, children: [tyre] });
  });
  b.roots(b.node({ name: "cart", children: [body, ...tyres] }));
  return b.glb();
}
