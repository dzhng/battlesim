// How an articulated bundle moves. The renderer's pose driver turns what a
// vehicle is doing into an `Articulation` (spike 03's pose inputs); this module
// maps those inputs onto the bundle's named nodes, for the renderer, the
// bake's posed bounds and the validator's deploy check alike.
//
// - The turret yaws about its local +Z; the gun pitches about its local +Y
//   and recoils back along its own bore (local −X). A model states its own
//   pitch limits and stroke on the gun node (`pitch_min_deg`,
//   `pitch_max_deg`, `recoil_max_m`).
// - The HMG yaws and pitches the same way, on its own, relative to the turret.
// - Wheels roll about their local +Y (the axle) by travel over radius, each
//   side by its own travel, so a tank turning in place counter-rotates them.
// - Tracks scroll their links by travel over link pitch (a material offset).
// - Rotors (`rotor_*`) turn about their local +Z by the distance their blade
//   tips have swept over their reach, so every rotor's tips run at one speed
//   and a small tail rotor turns faster than the main rotor.
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
  /** Metres the gun has run back along its bore after a shot; 0 in battery. */
  recoil: number;
  /** HMG heading relative to the turret. */
  hmg_yaw: number;
  hmg_pitch: number;
  /** Metres the left and right running gear have rolled forward. */
  travel_l: number;
  travel_r: number;
  /** Deployment progress: 0 packed, 1 deployed. */
  deploy: number;
  /** Metres every rotor's blade tips have swept: an accumulated angle times
   *  each rotor's reach. */
  rotor: number;
  /** 1: every rotor is drawn by its blur (as a camera's shutter sees it
   *  turn), not by its own geometry, which draws nothing; 0: by its geometry. */
  rotor_blur: number;
}

export const REST_ARTICULATION: Readonly<Articulation> = {
  turret_yaw: 0,
  gun_pitch: 0,
  recoil: 0,
  hmg_yaw: 0,
  hmg_pitch: 0,
  travel_l: 0,
  travel_r: 0,
  deploy: 0,
  rotor: 0,
  rotor_blur: 0,
};

const DEG = Math.PI / 180;

/** How far a rig's gun and HMG pitch, radians [down, up]. `articulate`
 *  stops the drawn guns at them and the bake's posed bounds sweep them. */
export interface PitchLimits {
  gun: readonly [number, number];
  hmg: readonly [number, number];
}

/** The pitch of a gun whose model states no limits of its own: a turret gun's
 *  and a roof HMG's. A model states its own on the pitching node, as custom
 *  properties `pitch_min_deg` and `pitch_max_deg` (a helicopter's chin gun
 *  depresses far below a tank gun). */
export const DEFAULT_PITCH_LIMITS: Readonly<PitchLimits> = {
  gun: [-10 * DEG, 20 * DEG],
  hmg: [-10 * DEG, 45 * DEG],
};

/** A pitching node's own limits, else `fallback`. */
function pitchOf(
  node: ArticulatedNode | undefined,
  fallback: readonly [number, number],
): readonly [number, number] {
  const e = node?.extras;
  if (e?.pitch_min_deg === undefined && e?.pitch_max_deg === undefined) return fallback;
  return [
    (e.pitch_min_deg ?? fallback[0] / DEG) * DEG,
    (e.pitch_max_deg ?? fallback[1] / DEG) * DEG,
  ];
}

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
  /** How far its gun and HMG pitch: their models' own limits, else the defaults. */
  pitch: PitchLimits;
  /** How far its gun can run back, metres: its node's `recoil_max_m` (an
   *  autocannon's short stroke), else unbounded. */
  stroke: number;
  wheels: { node: number; radius: number; left: boolean }[];
  tracks: { node: number; left: boolean; linkPitch: number }[];
  /** Each rotor, the reach of its blade tips from its axis, and its blades
   *  and mast as its geometry has them (`rotorShape`). */
  rotors: ({ node: number; radius: number } & RotorShape)[];
  deploy: DeployMotion[];
}

/** A rotor: a node the renderer turns about its local +Z, with its blades
 *  (`blade_*` meshes) under it. Its disc is not its airframe's size, so the
 *  hull fit leaves it out. */
export const isRotor = (name: string): boolean => /^(rotor|blade)_/.test(name);

/** Largest distance of a node's finest-tier vertex from its local axis
 *  through the origin along `axis` (1: +Y, a wheel's axle; 2: +Z, a rotor's mast). */
function reach(node: ArticulatedNode, axis: 1 | 2): number {
  const p = node.tiers[0].positions;
  const [a, b] = axis === 1 ? [0, 2] : [0, 1];
  let r2 = 0;
  for (let v = 0; v < p.length; v += 3)
    r2 = Math.max(r2, p[v + a] * p[v + a] + p[v + b] * p[v + b]);
  return Math.sqrt(r2);
}

/** A rotor's blades: how many, their width across (metres), and the angle
 *  one of them rests at about the rotor's +Z from its +X; and its mast and
 *  head: how far they run down and up its axis from the hub, and how thick
 *  the mast is (0: none). */
export interface RotorShape {
  blades: number;
  chord: number;
  rest: number;
  mast_below: number;
  mast_above: number;
  mast_radius: number;
}

/** The blades and mast in a rotor's geometry: its vertices out past
 *  `OUTER` of its reach (its blades, not its hub), grouped by angle round
 *  its axis, a group's width across its chord; and those near its axis,
 *  above and below its hub, its mast and head. */
function rotorShape(node: ArticulatedNode, radius: number): RotorShape {
  const p = node.tiers[0]?.positions ?? [];
  // Its mast and head: what stands near its axis, from the lowest to the
  // highest, and its thickness halfway down.
  let [low, high] = [Infinity, -Infinity];
  for (let v = 0; v < p.length; v += 3)
    if (Math.hypot(p[v], p[v + 1]) < MAST_REACH * radius)
      [low, high] = [Math.min(low, p[v + 2]), Math.max(high, p[v + 2])];
  let mast_radius = 0;
  for (let v = 0; v < p.length; v += 3)
    if (p[v + 2] < low / 2 && Math.hypot(p[v], p[v + 1]) < MAST_REACH * radius)
      mast_radius = Math.max(mast_radius, Math.hypot(p[v], p[v + 1]));
  const mast =
    mast_radius > 0
      ? { mast_below: -low, mast_above: Math.max(0, high), mast_radius }
      : { mast_below: 0, mast_above: 0, mast_radius: 0 };
  const at: { a: number; x: number; y: number }[] = [];
  for (let v = 0; v < p.length; v += 3)
    if (Math.hypot(p[v], p[v + 1]) > OUTER * radius)
      at.push({ a: Math.atan2(p[v + 1], p[v]), x: p[v], y: p[v + 1] });
  if (at.length === 0) return { blades: 0, chord: 0, rest: 0, ...mast };
  at.sort((m, n) => m.a - n.a);
  // Start a group after the widest gap, so none straddles the wrap.
  let start = 0;
  let widest = at[0].a + 2 * Math.PI - at[at.length - 1].a;
  for (let i = 1; i < at.length; i++)
    if (at[i].a - at[i - 1].a > widest) [widest, start] = [at[i].a - at[i - 1].a, i];
  const groups: (typeof at)[] = [];
  for (let k = 0; k < at.length; k++) {
    const v = at[(start + k) % at.length];
    const last = groups.at(-1);
    const prev = last?.at(-1);
    const gap = prev ? (v.a - prev.a + 2 * Math.PI) % (2 * Math.PI) : Infinity;
    if (last && gap < BLADE_GAP) last.push(v);
    else groups.push([v]);
  }
  const centre = (g: typeof at) =>
    Math.atan2(
      sum(g, (v) => v.y),
      sum(g, (v) => v.x),
    );
  const chord = Math.max(
    ...groups.map((g) => {
      const c = centre(g);
      const across = g.map((v) => -v.x * Math.sin(c) + v.y * Math.cos(c));
      return Math.max(...across) - Math.min(...across);
    }),
  );
  return { blades: groups.length, chord, rest: centre(groups[0]), ...mast };
}

const sum = <T>(xs: readonly T[], f: (x: T) => number) => xs.reduce((a, x) => a + f(x), 0);
/** How far out a rotor's vertices count as blade, a share of its reach. */
const OUTER = 0.6;
/** How near its axis a rotor's vertices count as mast, a share of its reach. */
const MAST_REACH = 0.05;
/** The least angle between two blades round a hub. */
const BLADE_GAP = (20 * Math.PI) / 180;

export function articulationRig(nodes: readonly ArticulatedNode[]): ArticulationRig {
  const find = (name: string) => nodes.findIndex((n) => n.name === name);
  const [gun, hmgGun] = [find("gun"), find("hmg_gun")];
  const rig: ArticulationRig = {
    turret: find("turret"),
    gun,
    hmg: find("hmg"),
    hmgGun,
    pitch: {
      gun: pitchOf(nodes[gun], DEFAULT_PITCH_LIMITS.gun),
      hmg: pitchOf(nodes[hmgGun], DEFAULT_PITCH_LIMITS.hmg),
    },
    stroke: nodes[gun]?.extras.recoil_max_m ?? Infinity,
    wheels: [],
    tracks: [],
    rotors: [],
    deploy: [],
  };
  nodes.forEach((node, i) => {
    if (node.name.startsWith("wheel_")) {
      const radius = node.extras.radius_m ?? reach(node, 1);
      if (radius > 0) rig.wheels.push({ node: i, radius, left: node.pivot[1] >= 0 });
    }
    if (node.name.startsWith("rotor_")) {
      const radius = node.extras.radius_m ?? reach(node, 2);
      if (radius > 0) rig.rotors.push({ node: i, radius, ...rotorShape(node, radius) });
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
const _articulate_bore = vec3.create();

/** Rotate `bind` about its own axis by `angle` into `out`. */
function turned(out: Trs, bind: Trs, axis: Vec3, angle: number): Trs {
  quat.setAxisAngle(_articulate_turn, axis, angle);
  quat.multiply(out.r, bind.r, _articulate_turn);
  return out;
}

const clampTo = (v: number, [lo, hi]: readonly [number, number]) => Math.min(hi, Math.max(lo, v));

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
  if (rig.gun >= 0) {
    const pitch = clampTo(input.gun_pitch, rig.pitch.gun);
    const gun = turned(out[rig.gun], nodes[rig.gun].bind, AXIS_Y, -pitch);
    // Recoil runs the gun back along its own bore, pitched with it.
    vec3.set(_articulate_bore, -Math.min(input.recoil, rig.stroke), 0, 0);
    vec3.transformQuat(_articulate_bore, _articulate_bore, gun.r);
    vec3.add(gun.t, gun.t, _articulate_bore);
  }
  if (rig.hmg >= 0) turned(out[rig.hmg], nodes[rig.hmg].bind, AXIS_Z, input.hmg_yaw);
  if (rig.hmgGun >= 0)
    turned(
      out[rig.hmgGun],
      nodes[rig.hmgGun].bind,
      AXIS_Y,
      -clampTo(input.hmg_pitch, rig.pitch.hmg),
    );
  for (const wheel of rig.wheels) {
    const travel = wheel.left ? input.travel_l : input.travel_r;
    turned(out[wheel.node], nodes[wheel.node].bind, AXIS_Y, travel / wheel.radius);
  }
  for (const rotor of rig.rotors) {
    turned(out[rotor.node], nodes[rotor.node].bind, AXIS_Z, input.rotor / rotor.radius);
    if (input.rotor_blur > 0) vec3.set(out[rotor.node].s, 0, 0, 0);
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
 * Articulations a rig's posed bounds sweep: deploy progress in quarters, and
 * at packed and deployed every sampled turret and HMG bearing at both of the
 * rig's pitch limits and level. Wheels and rotors are discs about their axis,
 * whose boxes `posedBounds` widens to hold every turn, so none is swept here.
 * A sampled turn misses at most a sagitta between bearings; `SWEEP_PAD` covers it.
 */
export function sweepArticulations(pitch: PitchLimits): Articulation[] {
  const out: Articulation[] = [];
  const turn = (n: number) => Array.from({ length: n }, (_, k) => (2 * Math.PI * k) / n);
  const rest = { ...REST_ARTICULATION };
  for (const deploy of [0.25, 0.5, 0.75]) out.push({ ...rest, deploy });
  for (const deploy of [0, 1])
    for (const turret_yaw of turn(SWEEP_BEARINGS.turret))
      for (const gun_pitch of [pitch.gun[0], 0, pitch.gun[1]])
        for (const hmg_yaw of turn(SWEEP_BEARINGS.hmg))
          for (const hmg_pitch of [pitch.hmg[0], 0, pitch.hmg[1]])
            out.push({ ...rest, turret_yaw, gun_pitch, hmg_yaw, hmg_pitch, deploy });
  return out;
}

/** Horizontal padding, per metre of reach from the vertical axis, that covers
 *  the arcs between the sweep's sampled bearings (the sagittas, summed). */
export const SWEEP_PAD =
  1 - Math.cos(Math.PI / SWEEP_BEARINGS.turret) + (1 - Math.cos(Math.PI / SWEEP_BEARINGS.hmg));
