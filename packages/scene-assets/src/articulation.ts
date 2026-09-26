// How an articulated bundle moves. The renderer's pose driver turns what a
// vehicle is doing into an `Articulation` (spike 03's pose inputs); this module
// maps those inputs onto the bundle's named nodes, for the renderer, the
// bake's posed bounds and the validator's deploy check alike.
//
// - The turret yaws about its local +Z; the gun pitches about its local +Y.
// - The HMG yaws and pitches the same way, on its own, relative to the turret.
// - Wheels roll about their local +Y (the axle) by travel over radius, each
//   side by its own travel, so a tank turning in place counter-rotates them.
// - Tracks scroll their links by travel over link pitch (a material offset).
// - Deploying parts move by their authored custom properties, over their own
//   window of deploy progress (see `DEPLOY_EXTRAS`).

import { quat, vec3, type Vec3 } from "math";
import type { Trs } from "./trs.ts";
import type { ArticulatedNode } from "./schema.ts";

export interface Articulation {
  /** Turret heading relative to the hull, radians counter-clockwise about +Z. */
  turret_yaw: number;
  /** Gun elevation, radians; positive raises the muzzle. */
  gun_pitch: number;
  /** HMG heading relative to the turret. */
  hmg_yaw: number;
  hmg_pitch: number;
  /** Metres the left and right running gear have rolled forward. */
  travel_l: number;
  travel_r: number;
  /** Deployment progress: 0 packed, 1 deployed. */
  deploy: number;
}

export const REST_ARTICULATION: Readonly<Articulation> = {
  turret_yaw: 0,
  gun_pitch: 0,
  hmg_yaw: 0,
  hmg_pitch: 0,
  travel_l: 0,
  travel_r: 0,
  deploy: 0,
};

const DEG = Math.PI / 180;

/** Presentation limits on the mounts' pitch; the pose driver clamps into them
 *  and the bake's posed bounds sweep them. */
export const PITCH_LIMITS = {
  gun: [-10 * DEG, 20 * DEG],
  hmg: [-10 * DEG, 45 * DEG],
} as const;

/**
 * A deploying node's custom properties (numbers, exported from Blender):
 * `deploy_start` and `deploy_end` bound its window of progress; at the end of
 * it the node has moved by `deploy_move_{x,y,z}` metres in its parent's frame
 * and turned by `deploy_turn_{x,y,z}` degrees about its own axes (x, then y,
 * then z). Progress moves it linearly through the window.
 */
export const DEPLOY_EXTRAS = [
  "deploy_start",
  "deploy_end",
  "deploy_move_x",
  "deploy_move_y",
  "deploy_move_z",
  "deploy_turn_x",
  "deploy_turn_y",
  "deploy_turn_z",
] as const;

export interface DeployMotion {
  node: number;
  start: number;
  end: number;
  move: Vec3;
  /** Degrees about the node's x, y, z axes. */
  turn: Vec3;
}

/** The moving parts of an articulated bundle, found once per bundle. */
export interface ArticulationRig {
  turret: number;
  gun: number;
  hmg: number;
  hmgGun: number;
  wheels: { node: number; radius: number; left: boolean }[];
  tracks: { node: number; left: boolean; linkPitch: number }[];
  deploy: DeployMotion[];
}

/** Largest distance of a node's finest-tier vertex from its axle (local +Y). */
function wheelRadius(node: ArticulatedNode): number {
  const p = node.tiers[0].positions;
  let r2 = 0;
  for (let v = 0; v < p.length; v += 3) r2 = Math.max(r2, p[v] * p[v] + p[v + 2] * p[v + 2]);
  return Math.sqrt(r2);
}

export function articulationRig(nodes: readonly ArticulatedNode[]): ArticulationRig {
  const find = (name: string) => nodes.findIndex((n) => n.name === name);
  const rig: ArticulationRig = {
    turret: find("turret"),
    gun: find("gun"),
    hmg: find("hmg"),
    hmgGun: find("hmg_gun"),
    wheels: [],
    tracks: [],
    deploy: [],
  };
  nodes.forEach((node, i) => {
    if (node.name.startsWith("wheel_")) {
      const radius = node.extras.radius_m ?? wheelRadius(node);
      if (radius > 0) rig.wheels.push({ node: i, radius, left: node.pivot[1] >= 0 });
    }
    if (/^track_[LR]$/.test(node.name) && node.extras.link_pitch_m > 0)
      rig.tracks.push({
        node: i,
        left: node.name === "track_L",
        linkPitch: node.extras.link_pitch_m,
      });
    const e = node.extras;
    if (e.deploy_start !== undefined && e.deploy_end !== undefined && e.deploy_end > e.deploy_start)
      rig.deploy.push({
        node: i,
        start: e.deploy_start,
        end: e.deploy_end,
        move: [e.deploy_move_x ?? 0, e.deploy_move_y ?? 0, e.deploy_move_z ?? 0],
        turn: [e.deploy_turn_x ?? 0, e.deploy_turn_y ?? 0, e.deploy_turn_z ?? 0],
      });
  });
  return rig;
}

const AXIS_Y: Vec3 = [0, 1, 0];
const AXIS_Z: Vec3 = [0, 0, 1];
const _articulate_turn = quat.create();

/** Rotate `bind` about its own axis by `angle` into `out`. */
function turned(out: Trs, bind: Trs, axis: Vec3, angle: number): Trs {
  quat.setAxisAngle(_articulate_turn, axis, angle);
  quat.multiply(out.r, bind.r, _articulate_turn);
  return out;
}

/** Fresh locals, one per node, holding each node's bind. */
export function restLocals(nodes: readonly ArticulatedNode[]): Trs[] {
  return nodes.map((n) => ({
    t: vec3.clone(n.bind.t),
    r: quat.clone(n.bind.r),
    s: vec3.clone(n.bind.s),
  }));
}

/** Pose every node's local transform for `input` into `out` (from `restLocals`). */
export function articulate(
  out: Trs[],
  nodes: readonly ArticulatedNode[],
  rig: ArticulationRig,
  input: Articulation,
): Trs[] {
  for (let i = 0; i < nodes.length; i++) {
    const bind = nodes[i].bind;
    vec3.copy(out[i].t, bind.t);
    quat.copy(out[i].r, bind.r);
    vec3.copy(out[i].s, bind.s);
  }
  if (rig.turret >= 0) turned(out[rig.turret], nodes[rig.turret].bind, AXIS_Z, input.turret_yaw);
  if (rig.gun >= 0) turned(out[rig.gun], nodes[rig.gun].bind, AXIS_Y, -input.gun_pitch);
  if (rig.hmg >= 0) turned(out[rig.hmg], nodes[rig.hmg].bind, AXIS_Z, input.hmg_yaw);
  if (rig.hmgGun >= 0) turned(out[rig.hmgGun], nodes[rig.hmgGun].bind, AXIS_Y, -input.hmg_pitch);
  for (const wheel of rig.wheels) {
    const travel = wheel.left ? input.travel_l : input.travel_r;
    turned(out[wheel.node], nodes[wheel.node].bind, AXIS_Y, travel / wheel.radius);
  }
  for (const motion of rig.deploy) {
    const s = Math.min(1, Math.max(0, (input.deploy - motion.start) / (motion.end - motion.start)));
    if (s === 0) continue;
    const local = out[motion.node];
    vec3.scaleAndAdd(local.t, local.t, motion.move, s);
    quat.fromDegrees(
      _articulate_turn,
      motion.turn[0] * s,
      motion.turn[1] * s,
      motion.turn[2] * s,
      "xyz",
    );
    quat.multiply(local.r, local.r, _articulate_turn);
  }
  return out;
}

/** Each track's link offset, in links (fractional), for the material's scroll. */
export function trackScroll(
  rig: ArticulationRig,
  input: Articulation,
): { left: number; right: number } {
  const of = (left: boolean) => {
    const track = rig.tracks.find((t) => t.left === left);
    return track ? (left ? input.travel_l : input.travel_r) / track.linkPitch : 0;
  };
  return { left: of(true), right: of(false) };
}

/** Bearings the posed-bounds sweep samples per full turn, turret and HMG. */
export const SWEEP_BEARINGS = { turret: 32, hmg: 16 } as const;

/**
 * Articulations the posed bounds sweep: deploy progress in quarters, and at
 * packed and deployed every sampled turret and HMG bearing at both pitch
 * limits and level. Wheels are discs about their axle, so their rest box
 * already holds every roll. A sampled turn misses at most a sagitta between
 * bearings; `SWEEP_PAD` covers it.
 */
export function sweepArticulations(): Articulation[] {
  const out: Articulation[] = [];
  const turn = (n: number) => Array.from({ length: n }, (_, k) => (2 * Math.PI * k) / n);
  const rest = { ...REST_ARTICULATION };
  for (const deploy of [0.25, 0.5, 0.75]) out.push({ ...rest, deploy });
  for (const deploy of [0, 1])
    for (const turret_yaw of turn(SWEEP_BEARINGS.turret))
      for (const gun_pitch of [PITCH_LIMITS.gun[0], 0, PITCH_LIMITS.gun[1]])
        for (const hmg_yaw of turn(SWEEP_BEARINGS.hmg))
          for (const hmg_pitch of [PITCH_LIMITS.hmg[0], 0, PITCH_LIMITS.hmg[1]])
            out.push({ ...rest, turret_yaw, gun_pitch, hmg_yaw, hmg_pitch, deploy });
  return out;
}

/** Horizontal padding, per metre of reach from the vertical axis, that covers
 *  the arcs between the sweep's sampled bearings (the sagittas, summed). */
export const SWEEP_PAD =
  1 - Math.cos(Math.PI / SWEEP_BEARINGS.turret) + (1 - Math.cos(Math.PI / SWEEP_BEARINGS.hmg));
