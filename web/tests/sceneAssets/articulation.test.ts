// @vitest-environment node
// The articulation seam: pose inputs (what the pose driver produces) mapped
// onto an articulated bundle's named nodes, and the posed culling bounds the
// bake derives from the same mapping.
import { expect, test } from "vitest";
import { vec3, type Vec3 } from "math";
import {
  PITCH_LIMITS,
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
import { AUTHORITY, TOLERANCES, tankGlb, truckGlb } from "./synthetic";

async function built(unit: "tank" | "supply", bytes: Uint8Array): Promise<ArticulatedBundle> {
  const result = await validateAppearance(
    {
      name: unit,
      entry: { unit: "vehicle", source: "a.glb", basis_yaw_deg: 0 },
      files: { "a.glb": bytes },
    },
    { authority: AUTHORITY, tolerances: TOLERANCES },
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
          PITCH_LIMITS.gun[0] + ((k * 0.13) % 1) * (PITCH_LIMITS.gun[1] - PITCH_LIMITS.gun[0]),
        recoil: (k % 3) * 0.2,
        hmg_yaw: -k * 0.61,
        hmg_pitch:
          PITCH_LIMITS.hmg[0] + ((k * 0.29) % 1) * (PITCH_LIMITS.hmg[1] - PITCH_LIMITS.hmg[0]),
        travel_l: k * 0.11,
        travel_r: -k * 0.07,
        deploy: (k % 11) / 10,
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
