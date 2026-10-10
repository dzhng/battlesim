// @vitest-environment node
// The articulation seam: pose inputs (what the pose driver produces) mapped
// onto an articulated bundle's named nodes, and the posed culling bounds the
// bake derives from the same mapping.
import { expect, test } from "vitest";
import { vec3, type Vec3 } from "math";
import {
  DEFAULT_PITCH_LIMITS,
  REST_ARTICULATION,
  articulate,
  articulationRig,
  restLocals,
  trackScroll,
  type Articulation,
} from "@packages/scene-assets/src/articulation.ts";
import {
  articulatedPositions,
  positionsBounds,
  posedBounds,
  worldTransforms,
} from "@packages/scene-assets/src/pose.ts";
import { pointAt } from "@packages/scene-assets/src/trs.ts";
import type { ArticulatedBundle } from "@packages/scene-assets/src/schema.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import {
  AUTHORITY,
  HELI_AUTHORITY,
  HELI_ROTORS,
  TANK_DRAWS,
  TOLERANCES,
  heliGlb,
  tankGlb,
  truckGlb,
} from "./synthetic";

async function built(
  unit: "tank" | "supply" | "heli",
  bytes: Uint8Array,
): Promise<ArticulatedBundle> {
  const result = await validateAppearance(
    {
      name: unit,
      entry: {
        unit: "vehicle",
        source: "a.glb",
        basis_yaw_deg: 0,
        ...(unit === "tank" ? { mounts: TANK_DRAWS } : {}),
      },
      files: { "a.glb": bytes },
    },
    { authority: unit === "heli" ? HELI_AUTHORITY : AUTHORITY, tolerances: TOLERANCES },
  );
  return result.preview as ArticulatedBundle;
}

function posed(bundle: ArticulatedBundle, input: Partial<Articulation>) {
  const nodes = bundle.nodes;
  const locals = articulate(restLocals(nodes), nodes, articulationRig(nodes), {
    ...REST_ARTICULATION,
    ...input,
  });
  const worlds = worldTransforms(
    nodes.map((n) => n.parent),
    locals,
  );
  const at = (name: string): Vec3 =>
    pointAt(worlds[nodes.findIndex((n) => n.name === name)], [0, 0, 0]);
  return { worlds, at, locals };
}

test("turret yaw carries the muzzle round the hull origin's vertical axis", async () => {
  const tank = await built("tank", tankGlb());
  const rest = posed(tank, {}).at("muzzle");
  for (const yaw of [0.3, Math.PI / 2, -2]) {
    const expected = vec3.rotateZ(vec3.create(), rest, [0, 0, 0], yaw);
    expect(vec3.distance(posed(tank, { turret_yaw: yaw }).at("muzzle"), expected)).toBeLessThan(
      1e-5,
    );
  }
});

test("positive gun pitch raises the muzzle and the HMG turns on its own", async () => {
  const tank = await built("tank", tankGlb());
  const rest = posed(tank, {});
  const raised = posed(tank, { gun_pitch: 0.2 });
  expect(raised.at("muzzle")[2]).toBeGreaterThan(rest.at("muzzle")[2] + 0.3);
  expect(raised.at("hmg_muzzle")).toEqual(rest.at("hmg_muzzle"));
  const hmg = posed(tank, { hmg_yaw: Math.PI / 2, hmg_pitch: 0.3 });
  expect(hmg.at("muzzle")).toEqual(rest.at("muzzle"));
  expect(hmg.at("hmg_muzzle")[1]).toBeGreaterThan(rest.at("hmg_muzzle")[1] + 0.5);
  expect(hmg.at("hmg_muzzle")[2]).toBeGreaterThan(rest.at("hmg_muzzle")[2]);
});

test("recoil runs the gun back along its own bore, level or raised", async () => {
  const tank = await built("tank", tankGlb());
  for (const gun_pitch of [0, 0.2]) {
    const battery = posed(tank, { gun_pitch });
    const back = posed(tank, { gun_pitch, recoil: 0.4 });
    const bore = vec3.normalize(
      vec3.create(),
      vec3.sub(vec3.create(), battery.at("muzzle"), battery.at("gun")),
    );
    const moved = vec3.sub(vec3.create(), back.at("muzzle"), battery.at("muzzle"));
    expect(vec3.distance(moved, vec3.scale(vec3.create(), bore, -0.4))).toBeLessThan(1e-5);
    // The turret and the HMG on it stay put.
    expect(back.at("hmg_muzzle")).toEqual(battery.at("hmg_muzzle"));
  }
});

test("a gun whose model states a short stroke runs back no further than it", async () => {
  // An autocannon's short recoil, against a tank gun's long one.
  const chin = await built("tank", tankGlb({ gunStrokeM: 0.08 }));
  const ran = (recoil: number) =>
    vec3.distance(posed(chin, { recoil }).at("muzzle"), posed(chin, {}).at("muzzle"));
  expect(ran(0.05)).toBeCloseTo(0.05, 5);
  expect(ran(0.4)).toBeCloseTo(0.08, 5);
});

test("each side's wheels roll by that side's travel over their radius", async () => {
  const tank = await built("tank", tankGlb());
  const rig = articulationRig(tank.nodes);
  const left = rig.wheels.find((w) => tank.nodes[w.node].name === "wheel_L_1")!;
  const right = rig.wheels.find((w) => tank.nodes[w.node].name === "wheel_R_1")!;
  expect(left.left).toBe(true);
  expect(right.left).toBe(false);
  // The synthetic wheel is a 0.7 m box: its corner reaches 0.35·√2 from the axle.
  expect(left.radius).toBeCloseTo(0.35 * Math.SQRT2, 5);
  const quarter = (left.radius * Math.PI) / 2;
  const { locals } = posed(tank, { travel_l: quarter });
  const spin = (node: number) => 2 * Math.acos(Math.min(1, Math.abs(locals[node].r[3])));
  expect(spin(left.node)).toBeCloseTo(Math.PI / 2, 5);
  expect(spin(right.node)).toBeCloseTo(0, 5);
  // Rolling forward brings the wheel's top toward +X.
  const top = vec3.transformQuat(vec3.create(), [0, 0, 1], locals[left.node].r);
  expect(top[0]).toBeCloseTo(1, 5);
});

test("tracks scroll by travel over link pitch, one side at a time", async () => {
  const tank = await built("tank", tankGlb());
  const rig = articulationRig(tank.nodes);
  expect(trackScroll(rig, { ...REST_ARTICULATION, travel_l: 0.8, travel_r: -0.16 })).toEqual({
    left: 0.8 / 0.16,
    right: -1,
  });
});

test("deploying slides the beams out, drops the pads to the ground and raises the mast", async () => {
  const truck = await built("supply", truckGlb());
  const packed = posed(truck, { deploy: 0 });
  const deployed = posed(truck, { deploy: 1 });
  const halfway = posed(truck, { deploy: 0.5 });
  expect(deployed.at("deploy_leg_FL")[1] - packed.at("deploy_leg_FL")[1]).toBeCloseTo(0.5, 5);
  expect(deployed.at("deploy_leg_FR")[1] - packed.at("deploy_leg_FR")[1]).toBeCloseTo(-0.5, 5);
  expect(packed.at("deploy_leg_FL_pad")[2]).toBeCloseTo(0.1, 5);
  expect(deployed.at("deploy_leg_FL_pad")[2]).toBeCloseTo(0, 5);
  const head = (p: ReturnType<typeof posed>) => p.at("deploy_mast_head")[2];
  expect(head(halfway)).toBeGreaterThan(head(packed));
  expect(head(deployed)).toBeGreaterThan(head(halfway));
  // The mast stands up: its head is straight above the hinge.
  expect(Math.abs(deployed.at("deploy_mast_head")[0] - deployed.at("deploy_mast")[0])).toBeLessThan(
    1e-4,
  );
});

test("posed bounds hold every reachable pose, the rest pose among them", async () => {
  for (const [unit, bytes] of [
    ["tank", tankGlb({ muzzleX: 5.9 })],
    ["supply", truckGlb()],
  ] as const) {
    const bundle = await built(unit, bytes);
    const bounds = posedBounds(bundle.nodes);
    expect(bundle.bounds).toEqual(bounds);
    const reached = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] };
    // Poses between the sweep's samples, not only on them.
    for (let k = 0; k < 40; k++) {
      const input: Articulation = {
        turret_yaw: k * 0.37,
        gun_pitch:
          DEFAULT_PITCH_LIMITS.gun[0] +
          ((k * 0.13) % 1) * (DEFAULT_PITCH_LIMITS.gun[1] - DEFAULT_PITCH_LIMITS.gun[0]),
        recoil: (k % 3) * 0.2,
        hmg_yaw: -k * 0.61,
        hmg_pitch:
          DEFAULT_PITCH_LIMITS.hmg[0] +
          ((k * 0.29) % 1) * (DEFAULT_PITCH_LIMITS.hmg[1] - DEFAULT_PITCH_LIMITS.hmg[0]),
        travel_l: k * 0.11,
        travel_r: -k * 0.07,
        deploy: (k % 11) / 10,
        rotor: k * 1.3,
        rotor_blur: 0,
      };
      const { worlds } = posed(bundle, input);
      positionsBounds(articulatedPositions(bundle.nodes, worlds, 0), reached as never);
    }
    for (let c = 0; c < 3; c++) {
      expect(reached.min[c]).toBeGreaterThanOrEqual(bounds.min[c] - 1e-4);
      expect(reached.max[c]).toBeLessThanOrEqual(bounds.max[c] + 1e-4);
    }
    if (unit === "tank") {
      // A turret swung astern carries the long gun behind the hull.
      expect(bounds.min[0]).toBeLessThan(-5.5);
    } else {
      // The raised mast, not the roof, sets the top.
      expect(bounds.max[2]).toBeGreaterThan(5);
    }
  }
});

test("every rotor turns about its own axis by the distance its blade tips sweep over their reach", async () => {
  const heli = await built("heli", heliGlb());
  const node = (name: string) => heli.nodes.findIndex((n) => n.name === name);
  const tipOf = (p: ReturnType<typeof posed>, rotor: string, reach: number) =>
    vec3.transformMat4(vec3.create(), [reach, 0, 0], p.worlds[node(rotor)]);
  // A quarter turn of the main rotor swings its tip from +X to +Y about the mast.
  const quarter = (HELI_ROTORS.main * Math.PI) / 2;
  const rest = posed(heli, {});
  const turned = posed(heli, { rotor: quarter });
  expect([...tipOf(rest, "rotor_main", 5)].map((v) => +v.toFixed(4))).toEqual([5.5, 0, 2.4]);
  expect([...tipOf(turned, "rotor_main", 5)].map((v) => +v.toFixed(4))).toEqual([0.5, 5, 2.4]);
  // The tail rotor's tips sweep as far, so the smaller rotor turns faster,
  // about its own axis (across the boom), never out of its plane.
  const hub = rest.at("rotor_tail");
  const [from, to] = [rest, turned].map((p) =>
    vec3.sub(vec3.create(), tipOf(p, "rotor_tail", 0.7), hub),
  );
  const swung = Math.acos(vec3.dot(from, to) / (vec3.length(from) * vec3.length(to)));
  const expected = (quarter / HELI_ROTORS.tail) % (2 * Math.PI);
  expect(swung).toBeCloseTo(Math.min(expected, 2 * Math.PI - expected), 4);
  expect(to[1]).toBeCloseTo(0, 5);
});

test("a rotor's whole disc is inside the culling bounds, whatever its angle", async () => {
  const heli = await built("heli", heliGlb());
  const bounds = posedBounds(heli.nodes);
  expect(heli.bounds).toEqual(bounds);
  for (let k = 0; k < 24; k++) {
    const { worlds } = posed(heli, { rotor: k * 0.83 });
    const b = positionsBounds(articulatedPositions(heli.nodes, worlds, 0));
    for (let c = 0; c < 3; c++) {
      expect(b.min[c]).toBeGreaterThanOrEqual(bounds.min[c] - 1e-4);
      expect(b.max[c]).toBeLessThanOrEqual(bounds.max[c] + 1e-4);
    }
  }
  // The disc, not the blade at rest along +X, sets the sides.
  expect(bounds.max[1]).toBeGreaterThan(5);
});

test("a gun whose model states its own pitch limits is drawn to them, and its bounds hold them", async () => {
  // A chin gun firing down from the air: its model lets it depress to -60°.
  const chin = await built("tank", tankGlb({ muzzleX: 5.9, gunPitchDeg: [-60, 11] }));
  const plain = await built("tank", tankGlb({ muzzleX: 5.9 }));
  const DEG = Math.PI / 180;
  // The drawn bore's elevation when the gun is asked for `pitch`.
  const drawn = (bundle: ArticulatedBundle, pitch: number) => {
    const p = posed(bundle, { gun_pitch: pitch });
    const d = vec3.sub(vec3.create(), p.at("muzzle"), p.at("gun"));
    return Math.atan2(d[2], Math.hypot(d[0], d[1]));
  };
  // Within its limits it goes where it is asked; past them, each gun stops at its own.
  expect(drawn(chin, -50 * DEG)).toBeCloseTo(-50 * DEG, 5);
  expect(drawn(chin, -80 * DEG)).toBeCloseTo(-60 * DEG, 5);
  expect(drawn(chin, 30 * DEG)).toBeCloseTo(11 * DEG, 5);
  expect(drawn(plain, -50 * DEG)).toBeCloseTo(DEFAULT_PITCH_LIMITS.gun[0], 5);
  // The bake's bounds reach the depressed muzzle on the gun that can depress that far.
  const low = posed(chin, { gun_pitch: -60 * DEG }).at("muzzle")[2];
  expect(posedBounds(chin.nodes).min[2]).toBeLessThanOrEqual(low + 1e-4);
  expect(posedBounds(plain.nodes).min[2]).toBeGreaterThan(low);
});

test("a rotor knows its blades from its own geometry: how many, how wide, and where they rest", async () => {
  const heli = await built("heli", heliGlb());
  const rig = articulationRig(heli.nodes);
  const rotor = (name: string) => rig.rotors.find((r) => heli.nodes[r.node].name === name)!;
  // The main rotor is one bar through its hub: two blades, 0.3 m wide,
  // resting along its +X and -X.
  const main = rotor("rotor_main");
  expect(main.blades).toBe(2);
  expect(main.chord).toBeCloseTo(0.3, 2);
  expect(Math.abs(Math.cos(main.rest))).toBeCloseTo(1, 4);
  // Its mast runs 0.2 m down from the hub, 0.16 m thick.
  expect(main.mast_below).toBeCloseTo(0.2, 4);
  expect(main.mast_radius).toBeCloseTo(Math.hypot(0.08, 0.08), 4);
  // The tail rotor has none.
  expect(rotor("rotor_tail").mast_radius).toBe(0);
  expect(rotor("rotor_tail").blades).toBe(2);
});

test("a rotor drawn by its blur draws none of its own geometry", async () => {
  const heli = await built("heli", heliGlb());
  const nodes = heli.nodes;
  const rig = articulationRig(nodes);
  const locals = articulate(restLocals(nodes), nodes, rig, { ...REST_ARTICULATION, rotor_blur: 1 });
  for (const r of rig.rotors) expect([...locals[r.node].s]).toEqual([0, 0, 0]);
  const shown = articulate(restLocals(nodes), nodes, rig, REST_ARTICULATION);
  for (const r of rig.rotors) expect([...shown[r.node].s]).not.toEqual([0, 0, 0]);
});
