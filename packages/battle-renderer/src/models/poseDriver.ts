// The pose driver: the one place a pose is derived from what the simulation
// published (README "Single owners"). It reads a feed frame — each unit's
// soldiers by id and position, its mounts' bearings, elevations and shot
// counts, deployment progress, suppression, and the fallen — and produces a
// `PoseFrame`: one pose per living or dying soldier, one articulation per
// vehicle, and the static corpses.
//
// Everything is per soldier: a soldier's gait and facing come from his own
// velocity, never from a formation slot or the squad's heading, so soldiers
// that move on their own (the future Company of Heroes-style spec) need no
// change here. A soldier's own posture, when the feed carries one, picks
// kneeling or prone directly. The battle fills `FeedFrame` from the decoded
// observation (`apps/battle-lab/src/poseFeed.ts`); the workbench fills it from
// synthetic frames.
//
// The frame is rewritten in place by every `update`: poses are the driver's
// own objects, valid until the next call. Corpses are the one population that
// can grow into the tens of thousands, so they are reconciled only when the
// feed hands over a new `fallen` list, and `corpsesVersion` says when the
// static list changed.

import { clamp, deltaAngle, vec3, type Vec3 } from "math";
import { mulberry32 } from "math/random";
import {
  PITCH_LIMITS,
  REST_ARTICULATION,
  type Articulation,
} from "@packages/scene-assets/src/articulation";
import type { Side } from "@packages/scene-assets/src/schema";

export type UnitKindName = "rifle" | "recon" | "at" | "tank" | "supply" | "jeep";
export type Posture = "stand" | "kneel" | "prone";

export interface FeedSoldier {
  id: number;
  position: Vec3;
  /** The soldier's own posture, when the simulation publishes one. */
  posture?: Posture;
  /** A round of his is in this tick's visible flight: when the squad's shot
   *  counter rises, the soldiers so named are the ones who fired. */
  shooting?: boolean;
}

export interface FeedMount {
  /** World bearing, radians. */
  bearing: number;
  elevation: number;
  /** Rounds launched so far; a rise is a shot. */
  shots: number;
}

export interface FeedUnit {
  id: number;
  kind: UnitKindName;
  side: Side;
  position: Vec3;
  /** Hull heading, radians counter-clockwise from +X. */
  yaw: number;
  soldiers: FeedSoldier[];
  mounts: FeedMount[];
  /** Deployment progress, null for units that do not deploy. */
  deployment: number | null;
  /** Infantry suppression in [0, 1] (0 where the side cannot know it). */
  suppression: number;
}

export interface FeedFallen {
  soldier: number;
  position: Vec3;
  /** The published heading of the fallen (his squad's, when he fell). */
  yaw: number;
  /** The kind of squad he fought in, for his appearance. */
  kind: UnitKindName;
  side: Side;
}

/** One published moment, in simulation seconds. */
export interface FeedFrame {
  time: number;
  units: FeedUnit[];
  /** Everyone the side knows has fallen. Hand over the same array while it is
   *  unchanged: the driver reconciles corpses only when it changes. */
  fallen: readonly FeedFallen[];
}

export interface ClipBlend {
  clip: string;
  phase: number;
  /** Weight of this clip over the soldier's main clip, 0..1. */
  weight: number;
}

export interface SoldierPose {
  soldier: number;
  /** His unit's id; -1 once he has fallen. */
  unit: number;
  kind: UnitKindName;
  side: Side;
  position: Vec3;
  /** Heading of the body's +X, radians. */
  facing: number;
  clip: string;
  phase: number;
  /** The clip being faded out, if a transition is under way. */
  blend: ClipBlend | null;
}

export interface VehiclePose {
  unit: number;
  kind: UnitKindName;
  side: Side;
  position: Vec3;
  yaw: number;
  articulation: Articulation;
}

/** A fallen soldier whose death has played out (or was never seen): drawn
 *  as a static mesh, never skinned. */
export interface CorpsePose {
  soldier: number;
  kind: UnitKindName;
  side: Side;
  position: Vec3;
  yaw: number;
}

export interface PoseFrame {
  soldiers: SoldierPose[];
  vehicles: VehiclePose[];
  corpses: CorpsePose[];
  /** Rises whenever `corpses` changed. */
  corpsesVersion: number;
}

/** What a mount is, by its index in the unit kind's mount list. */
export type MountRole = "gun" | "hmg" | "hand";

/** What the driver needs to know about the clips it picks. */
export interface ClipFacts {
  duration: number;
  loop: boolean;
  stride_m: number | null;
}

export interface PoseDriverOptions {
  /** Mount roles per unit kind, in the rules' mount order. */
  mounts: Partial<Record<UnitKindName, MountRole[]>>;
  /** A kind's clip durations and strides, for phase; null for a clip its rig lacks. */
  clip: (kind: UnitKindName, name: string) => ClipFacts | null;
  /** Half the distance between a vehicle kind's tracks or wheel rows, metres. */
  halfTrack: Partial<Record<UnitKindName, number>>;
  /** Suppression at which a soldier with no posture of his own goes prone:
   *  the rules' `suppression.collapse_level`. */
  pinned: number;
}

/** Gait thresholds (m/s) and timings (s): presentation, not rules. */
export const GAIT = {
  walk: 0.25,
  run: 2.2,
  /** Seconds a soldier stays in his firing pose after a shot. */
  firing: 1.5,
  /** Crossfade between clips. */
  fade: 0.25,
  /** Speed below which facing does not follow velocity. */
  facing: 0.3,
  /** Turn rate toward a new facing, radians per second. */
  turn: 6,
} as const;

/** Where a soldier starts a looping clip: his own offset (the golden ratio
 *  over his id), so a squad that starts walking, kneels or breathes together
 *  never moves in lockstep. A one-shot clip (death) starts at its start. */
export function loopStart(soldier: number): number {
  return (soldier * 0.6180339887498949) % 1;
}

/** How a soldier stands at rest, his own for life (from his id): how far his
 *  gaze strays from the squad's aim (radians, within ±`REST.turn`), how fast
 *  his idle plays (within `REST.tempo`), and whether he stands watching, his
 *  weapon up (`stand_aim`, one man in `REST.watch`), rather than at ease. A
 *  squad at rest scans different ways, in different stances, out of step,
 *  never a row of copies; while his squad is shooting every man faces the
 *  aim, and the gaze strays again only once the squad has been quiet for
 *  `REST.settle` seconds. Presentation, not rules. */
export function restManner(soldier: number): { turn: number; tempo: number; watch: boolean } {
  const state = mulberry32.create(Math.imul(soldier + 1, 0x9e3779b1) >>> 0);
  const turn = (mulberry32.sample(state) * 2 - 1) * REST.turn;
  const tempo = REST.tempo[0] + mulberry32.sample(state) * (REST.tempo[1] - REST.tempo[0]);
  // every `REST.watch`-th man by id, so each squad mixes its stances
  const watch = soldier % REST.watch === 2;
  return { turn, tempo, watch };
}

export const REST = {
  turn: 1.0,
  tempo: [0.8, 1.25],
  watch: 4,
  /** Seconds after the squad's last shot before the gaze starts to stray, and
   *  over which it strays fully. */
  settle: [4, 6],
} as const;

interface SoldierState {
  pose: SoldierPose;
  /** His `restManner`, and when his squad last fired. */
  turn: number;
  tempo: number;
  watch: boolean;
  alertAt: number;
  /** The blend `pose.blend` points at while a fade runs. */
  fading: ClipBlend;
  fadeLeft: number;
  lastShots: number;
  firedAt: number;
  /** When he fell, once the feed lists him among the fallen. */
  fellAt: number | null;
  seen: number;
}

/** How a vehicle's mounts move between what the simulation publishes:
 *  presentation feel, not rules (the slice's delegated recoil feel). */
export const MOUNT_FEEL = {
  /** A mount's published elevation is its last round's, so it changes only
   *  on a shot; the gun eases to it at up to this rate, radians per second. */
  gunElevationRate: 0.6,
  hmgElevationRate: 2,
  /** How far the gun runs back on a shot, metres, and how long it takes to
   *  run out to battery again, seconds. */
  recoilM: 0.45,
  recoilReturnS: 0.9,
} as const;

interface VehicleState {
  pose: VehiclePose;
  /** The gun's shot counter as last seen, and when its last shot was. */
  gunShots: number;
  firedAt: number;
  seen: number;
}

/** `from` moved toward `to` by at most `step`. */
function approach(from: number, to: number, step: number): number {
  const d = to - from;
  return Math.abs(d) <= step ? to : from + Math.sign(d) * step;
}

/** The gun's run-back `since` seconds after a shot: all the way back at once,
 *  then out to battery, fast at first and settling. */
export function recoilAt(since: number): number {
  if (!(since >= 0) || since >= MOUNT_FEEL.recoilReturnS) return 0;
  const left = 1 - since / MOUNT_FEEL.recoilReturnS;
  return MOUNT_FEEL.recoilM * left * left;
}

export class PoseDriver {
  private readonly soldiers = new Map<number, SoldierState>();
  private readonly vehicles = new Map<number, VehicleState>();
  /** Soldiers playing their death, by id (a subset of `soldiers`). */
  private readonly dying = new Set<number>();
  private readonly corpseMap = new Map<number, CorpsePose>();
  private lastFallen: readonly FeedFallen[] | null = null;
  private time: number | null = null;
  private generation = 0;
  private readonly out: PoseFrame = { soldiers: [], vehicles: [], corpses: [], corpsesVersion: 0 };

  constructor(private readonly options: PoseDriverOptions) {}

  /** Forget everything: the next frame starts fresh, as if first seen. */
  reset() {
    this.soldiers.clear();
    this.vehicles.clear();
    this.dying.clear();
    this.corpseMap.clear();
    this.lastFallen = null;
    this.time = null;
    this.out.corpses.length = 0;
    this.out.corpsesVersion++;
  }

  /** Advance to `frame` and pose everything in it. */
  update(frame: FeedFrame): PoseFrame {
    // Time running backwards is a new battle (a reset or a reloaded replay).
    if (this.time !== null && frame.time < this.time) this.reset();
    const dt = this.time === null ? 0 : Math.max(0, frame.time - this.time);
    this.time = frame.time;
    const generation = ++this.generation;
    const out = this.out;
    out.soldiers.length = 0;
    out.vehicles.length = 0;
    for (const unit of frame.units) {
      if (unit.kind === "tank" || unit.kind === "supply" || unit.kind === "jeep")
        out.vehicles.push(this.vehicle(unit, frame.time, dt, generation));
      else this.squad(unit, frame.time, dt, generation);
    }
    if (frame.fallen !== this.lastFallen) {
      this.lastFallen = frame.fallen;
      this.reconcileFallen(frame.fallen, frame.time);
    }
    this.advanceDying(frame.time, dt, generation);
    for (const [id, s] of this.soldiers)
      if (s.seen !== generation) {
        this.soldiers.delete(id);
        this.dying.delete(id);
      }
    for (const [id, v] of this.vehicles) if (v.seen !== generation) this.vehicles.delete(id);
    return out;
  }

  private squad(unit: FeedUnit, time: number, dt: number, generation: number) {
    const roles = this.options.mounts[unit.kind] ?? [];
    let shots = 0;
    let aim = unit.yaw;
    let aimed = false;
    for (let i = 0; i < unit.mounts.length; i++) {
      if (roles[i] !== "hand") continue;
      const mount = unit.mounts[i];
      shots += mount.shots;
      if (!aimed && mount.shots > 0) {
        // A hand weapon's bearing is its last aim, meaningful once it has fired.
        aim = mount.bearing;
        aimed = true;
      }
    }
    let named = false;
    for (const s of unit.soldiers) named ||= s.shooting === true;
    for (const soldier of unit.soldiers) {
      const state =
        this.soldiers.get(soldier.id) ?? this.newSoldier(soldier, unit, shots, generation);
      state.seen = generation;
      const pose = state.pose;
      const dx = soldier.position[0] - pose.position[0];
      const dy = soldier.position[1] - pose.position[1];
      const moved = Math.hypot(dx, dy);
      const speed = dt > 0 ? moved / dt : 0;
      // A rise of the squad's counter is a shot by whoever the visible rounds
      // name; with none named, by the whole squad.
      if (shots > state.lastShots && (!named || soldier.shooting === true)) state.firedAt = time;
      if (shots > state.lastShots) state.alertAt = time;
      state.lastShots = shots;
      const firing = time - state.firedAt < GAIT.firing;

      // Facing: his own velocity, else the weapon's aim (or the unit's heading),
      // strayed by his own manner once the squad has settled.
      const settled = clamp((time - state.alertAt - REST.settle[0]) / REST.settle[1], 0, 1);
      const target = speed > GAIT.facing ? Math.atan2(dy, dx) : aim + state.turn * settled;
      const turn = deltaAngle(pose.facing, target);
      const step = GAIT.turn * dt;
      pose.facing += Math.abs(turn) <= step ? turn : Math.sign(turn) * step;

      const posture =
        soldier.posture ??
        (unit.suppression >= this.options.pinned ? "prone" : firing ? "kneel" : "stand");
      const clip =
        posture === "prone"
          ? "prone_pinned"
          : posture === "kneel" && speed < GAIT.walk
            ? "kneel_fire"
            : speed >= GAIT.run
              ? "run"
              : speed >= GAIT.walk
                ? "walk"
                : state.watch
                  ? "stand_aim"
                  : "idle";
      this.advance(state, clip, moved, dt);
      vec3.copy(pose.position, soldier.position);
      pose.unit = unit.id;
      this.out.soldiers.push(pose);
    }
  }

  private newSoldier(
    soldier: FeedSoldier,
    unit: FeedUnit,
    shots: number,
    generation: number,
  ): SoldierState {
    const { turn, tempo, watch } = restManner(soldier.id);
    const state: SoldierState = {
      pose: {
        soldier: soldier.id,
        unit: unit.id,
        kind: unit.kind,
        side: unit.side,
        position: vec3.clone(soldier.position),
        facing: unit.yaw + turn,
        clip: "idle",
        phase: loopStart(soldier.id),
        blend: null,
      },
      turn,
      tempo,
      watch,
      alertAt: -Infinity,
      fading: { clip: "idle", phase: 0, weight: 0 },
      fadeLeft: 0,
      // Rounds fired before he was first seen are not his shot.
      lastShots: shots,
      firedAt: -Infinity,
      fellAt: null,
      seen: generation,
    };
    this.soldiers.set(soldier.id, state);
    return state;
  }

  /** A new `fallen` list: soldiers seen alive start their death; the rest
   *  (and anyone first seen already down) lie as corpses at once. */
  private reconcileFallen(fallen: readonly FeedFallen[], time: number) {
    let changed = false;
    const listed = new Set<number>();
    for (const f of fallen) {
      listed.add(f.soldier);
      if (this.corpseMap.has(f.soldier)) continue;
      const state = this.soldiers.get(f.soldier);
      if (state && state.fellAt === null) {
        state.fellAt = time;
        vec3.copy(state.pose.position, f.position);
        state.pose.unit = -1;
        this.switchTo(state, "death");
        this.dying.add(f.soldier);
      } else if (!state) {
        this.corpseMap.set(f.soldier, {
          soldier: f.soldier,
          kind: f.kind,
          side: f.side,
          position: vec3.clone(f.position),
          yaw: f.yaw,
        });
        changed = true;
      }
    }
    for (const id of this.corpseMap.keys())
      if (!listed.has(id)) {
        this.corpseMap.delete(id);
        changed = true;
      }
    for (const id of this.dying)
      if (!listed.has(id)) {
        this.dying.delete(id);
        this.soldiers.delete(id);
      }
    if (changed) this.publishCorpses();
  }

  /** Play each death on; a finished one becomes a static corpse. */
  private advanceDying(time: number, dt: number, generation: number) {
    for (const id of this.dying) {
      const state = this.soldiers.get(id)!;
      state.seen = generation;
      const pose = state.pose;
      const facts = this.options.clip(pose.kind, "death");
      pose.phase = facts ? Math.min(1, (time - state.fellAt!) / facts.duration) : 1;
      this.fade(state, dt);
      if (pose.phase >= 1 && !pose.blend) {
        this.dying.delete(id);
        this.soldiers.delete(id);
        this.corpseMap.set(id, {
          soldier: id,
          kind: pose.kind,
          side: pose.side,
          position: vec3.clone(pose.position),
          yaw: pose.facing,
        });
        this.publishCorpses();
        continue;
      }
      this.out.soldiers.push(pose);
    }
  }

  private publishCorpses() {
    const corpses = this.out.corpses;
    corpses.length = 0;
    for (const c of this.corpseMap.values()) corpses.push(c);
    this.out.corpsesVersion++;
  }

  /** Move to `clip` (fading from the current one) and advance its phase. */
  private advance(state: SoldierState, clip: string, moved: number, dt: number) {
    const pose = state.pose;
    if (clip !== pose.clip) this.switchTo(state, clip);
    const facts = this.options.clip(pose.kind, pose.clip);
    if (facts) {
      // Locomotion with a declared stride advances with ground covered, so
      // feet do not slide; everything else advances with time, the idle at
      // his own tempo.
      const step =
        facts.stride_m && (pose.clip === "walk" || pose.clip === "run")
          ? moved / facts.stride_m
          : (dt / facts.duration) * (pose.clip === "idle" ? state.tempo : 1);
      pose.phase = facts.loop ? (pose.phase + step) % 1 : Math.min(1, pose.phase + step);
    }
    this.fade(state, dt);
  }

  private switchTo(state: SoldierState, clip: string) {
    const pose = state.pose;
    state.fading.clip = pose.clip;
    state.fading.phase = pose.phase;
    state.fading.weight = 1;
    pose.blend = state.fading;
    state.fadeLeft = GAIT.fade;
    pose.clip = clip;
    pose.phase = this.options.clip(pose.kind, clip)?.loop ? loopStart(pose.soldier) : 0;
  }

  private fade(state: SoldierState, dt: number) {
    const pose = state.pose;
    if (!pose.blend) return;
    state.fadeLeft = Math.max(0, state.fadeLeft - dt);
    state.fading.weight = state.fadeLeft / GAIT.fade;
    if (state.fading.weight <= 0) pose.blend = null;
  }

  private vehicle(unit: FeedUnit, time: number, dt: number, generation: number): VehiclePose {
    const roles = this.options.mounts[unit.kind] ?? [];
    const gun = roles.indexOf("gun");
    const hmg = roles.indexOf("hmg");
    const gunMount = gun >= 0 ? unit.mounts[gun] : undefined;
    const hmgMount = hmg >= 0 ? unit.mounts[hmg] : undefined;
    const gunTarget = gunMount
      ? clamp(gunMount.elevation, PITCH_LIMITS.gun[0], PITCH_LIMITS.gun[1])
      : 0;
    const hmgTarget = hmgMount
      ? clamp(hmgMount.elevation, PITCH_LIMITS.hmg[0], PITCH_LIMITS.hmg[1])
      : 0;
    let state = this.vehicles.get(unit.id);
    if (!state) {
      // First seen: posed as published, with no shot to recoil from.
      state = {
        gunShots: gunMount?.shots ?? 0,
        firedAt: -Infinity,
        pose: {
          unit: unit.id,
          kind: unit.kind,
          side: unit.side,
          position: vec3.clone(unit.position),
          yaw: unit.yaw,
          articulation: { ...REST_ARTICULATION, gun_pitch: gunTarget, hmg_pitch: hmgTarget },
        },
        seen: generation,
      };
      this.vehicles.set(unit.id, state);
    }
    state.seen = generation;
    const pose = state.pose;
    const a = pose.articulation;
    // Ground covered along the hull, plus each side's share of the turn.
    const forward =
      (unit.position[0] - pose.position[0]) * Math.cos(unit.yaw) +
      (unit.position[1] - pose.position[1]) * Math.sin(unit.yaw);
    const turned = deltaAngle(pose.yaw, unit.yaw);
    const half = this.options.halfTrack[unit.kind] ?? 0;
    a.travel_l += forward - turned * half;
    a.travel_r += forward + turned * half;
    vec3.copy(pose.position, unit.position);
    pose.yaw = unit.yaw;
    a.deploy = unit.deployment ?? 0;

    // The turret on the cannon's bearing, the HMG relative to the turret it
    // rides; elevations eased (they change only on a shot); a new cannon
    // round recoils the gun.
    const turretBearing = gunMount ? gunMount.bearing : unit.yaw;
    a.turret_yaw = gunMount ? deltaAngle(unit.yaw, gunMount.bearing) : 0;
    a.gun_pitch = approach(a.gun_pitch, gunTarget, MOUNT_FEEL.gunElevationRate * dt);
    a.hmg_yaw = hmgMount ? deltaAngle(turretBearing, hmgMount.bearing) : 0;
    a.hmg_pitch = approach(a.hmg_pitch, hmgTarget, MOUNT_FEEL.hmgElevationRate * dt);
    if (gunMount && gunMount.shots > state.gunShots) state.firedAt = time;
    if (gunMount) state.gunShots = gunMount.shots;
    a.recoil = recoilAt(time - state.firedAt);
    return pose;
  }
}
