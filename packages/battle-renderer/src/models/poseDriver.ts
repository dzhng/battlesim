// The pose driver: the one place a pose is derived from what the simulation
// published (README "Single owners"). It reads a feed frame — each unit's
// soldiers by id and position, its mounts' bearings, elevations and shot
// counts, deployment progress, suppression, and the fallen — and produces a
// `PoseFrame`: one pose per soldier and one articulation per vehicle.
//
// Everything is per soldier: a soldier's gait and facing come from his own
// velocity, never from a formation slot or the squad's heading, so soldiers
// that move on their own (the future Company of Heroes-style spec) need no
// change here. A soldier's own posture, when the feed carries one, picks
// kneeling or prone directly. The battle fills `FeedFrame` from the decoded
// observation (slices 23–24); the workbench fills it from synthetic frames.

import { clamp, deltaAngle, vec2, type Vec3 } from "math";
import {
  PITCH_LIMITS,
  REST_ARTICULATION,
  type Articulation,
} from "@packages/scene-assets/src/articulation";

export type UnitKindName = "rifle" | "recon" | "at" | "tank" | "supply";
export type Posture = "stand" | "kneel" | "prone";

export interface FeedSoldier {
  id: number;
  position: Vec3;
  /** The soldier's own posture, when the simulation publishes one. */
  posture?: Posture;
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
  position: Vec3;
  /** Hull heading, radians counter-clockwise from +X. */
  yaw: number;
  soldiers: FeedSoldier[];
  mounts: FeedMount[];
  /** Deployment progress, null for units that do not deploy. */
  deployment: number | null;
  /** Infantry suppression in [0, 1]. */
  suppression: number;
}

export interface FeedFallen {
  soldier: number;
  position: Vec3;
  yaw: number;
  /** The kind of squad he fought in, for his appearance. */
  kind: UnitKindName;
}

/** One published moment, in simulation seconds. */
export interface FeedFrame {
  time: number;
  units: FeedUnit[];
  fallen: FeedFallen[];
}

export interface ClipBlend {
  clip: string;
  phase: number;
  /** Weight of this clip over the soldier's main clip, 0..1. */
  weight: number;
}

export interface SoldierPose {
  soldier: number;
  unit: number;
  kind: UnitKindName;
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
  position: Vec3;
  yaw: number;
  articulation: Articulation;
}

export interface PoseFrame {
  soldiers: SoldierPose[];
  vehicles: VehiclePose[];
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
  /** Clip durations and strides, for phase; null for a clip the rig lacks. */
  clip: (name: string) => ClipFacts | null;
  /** Half the distance between a vehicle kind's tracks or wheel rows, metres. */
  halfTrack: Partial<Record<UnitKindName, number>>;
}

/** Gait thresholds (m/s) and timings (s): presentation, not rules. */
export const GAIT = {
  walk: 0.25,
  run: 2.2,
  /** Seconds a soldier stays in his firing pose after a shot. */
  firing: 1.5,
  /** Suppression at which an unposed soldier goes prone. */
  pinned: 0.6,
  /** Crossfade between clips. */
  fade: 0.25,
  /** Speed below which facing holds instead of following velocity. */
  facing: 0.3,
  /** Turn rate toward a new facing, radians per second. */
  turn: 6,
} as const;

interface SoldierState {
  position: Vec3;
  facing: number;
  clip: string;
  phase: number;
  previous: ClipBlend | null;
  fadeLeft: number;
  lastShots: number;
  firedAt: number;
  fellAt: number | null;
  seen: number;
}

interface VehicleState {
  position: Vec3;
  yaw: number;
  travelL: number;
  travelR: number;
  seen: number;
}

const _driver_step = vec2.create();

export class PoseDriver {
  private readonly soldiers = new Map<number, SoldierState>();
  private readonly vehicles = new Map<number, VehicleState>();
  private time: number | null = null;
  private generation = 0;

  constructor(private readonly options: PoseDriverOptions) {}

  /** Forget every soldier and vehicle: the next frame starts fresh. */
  reset() {
    this.soldiers.clear();
    this.vehicles.clear();
    this.time = null;
  }

  /** Advance to `frame` and pose everything in it. */
  update(frame: FeedFrame): PoseFrame {
    const dt = this.time === null ? 0 : Math.max(0, frame.time - this.time);
    this.time = frame.time;
    const generation = ++this.generation;
    const out: PoseFrame = { soldiers: [], vehicles: [] };
    for (const unit of frame.units) {
      if (unit.kind === "tank" || unit.kind === "supply")
        out.vehicles.push(this.vehicle(unit, generation));
      else
        for (const soldier of unit.soldiers)
          out.soldiers.push(this.soldier(unit, soldier, frame.time, dt, generation));
    }
    for (const fallen of frame.fallen)
      out.soldiers.push(this.fallen(fallen, frame.time, dt, generation));
    for (const [id, s] of this.soldiers) if (s.seen !== generation) this.soldiers.delete(id);
    for (const [id, v] of this.vehicles) if (v.seen !== generation) this.vehicles.delete(id);
    return out;
  }

  private soldier(
    unit: FeedUnit,
    soldier: FeedSoldier,
    time: number,
    dt: number,
    generation: number,
  ): SoldierPose {
    const roles = this.options.mounts[unit.kind] ?? [];
    const shots = unit.mounts.reduce((n, m, i) => (roles[i] === "hand" ? n + m.shots : n), 0);
    const state =
      this.soldiers.get(soldier.id) ?? this.newSoldier(soldier, unit, shots, generation);
    state.seen = generation;
    vec2.set(
      _driver_step,
      soldier.position[0] - state.position[0],
      soldier.position[1] - state.position[1],
    );
    const moved = vec2.length(_driver_step);
    const speed = dt > 0 ? moved / dt : 0;
    if (shots > state.lastShots) state.firedAt = time;
    state.lastShots = shots;
    const firing = time - state.firedAt < GAIT.firing;
    const aim = unit.mounts.find((_, i) => roles[i] === "hand")?.bearing ?? unit.yaw;

    // Facing: along his own motion, else toward his aim while firing, else held.
    let target = state.facing;
    if (speed > GAIT.facing) target = Math.atan2(_driver_step[1], _driver_step[0]);
    else if (firing) target = aim;
    const turn = deltaAngle(state.facing, target);
    const step = GAIT.turn * dt;
    state.facing += Math.abs(turn) <= step ? turn : Math.sign(turn) * step;

    const posture =
      soldier.posture ?? (unit.suppression >= GAIT.pinned ? "prone" : firing ? "kneel" : "stand");
    const clip =
      posture === "prone"
        ? "prone_pinned"
        : posture === "kneel" && speed < GAIT.walk
          ? "kneel_fire"
          : speed >= GAIT.run
            ? "run"
            : speed >= GAIT.walk
              ? "walk"
              : "idle";
    this.advance(state, clip, moved, dt);
    state.position = [soldier.position[0], soldier.position[1], soldier.position[2]];
    return this.soldierPose(state, soldier.id, unit.id, unit.kind);
  }

  private fallen(fallen: FeedFallen, time: number, dt: number, generation: number): SoldierPose {
    let state = this.soldiers.get(fallen.soldier);
    if (!state) {
      // Seen only once already down: he lies as a corpse, the clip's end.
      state = {
        position: [...fallen.position] as Vec3,
        facing: fallen.yaw,
        clip: "death",
        phase: 1,
        previous: null,
        fadeLeft: 0,
        lastShots: 0,
        firedAt: -Infinity,
        fellAt: -Infinity,
        seen: generation,
      };
      this.soldiers.set(fallen.soldier, state);
    }
    state.seen = generation;
    if (state.fellAt === null) {
      state.fellAt = time;
      state.facing = fallen.yaw;
      this.switchTo(state, "death");
    }
    const facts = this.options.clip("death");
    state.phase = facts ? Math.min(1, (time - state.fellAt) / facts.duration) : 1;
    this.fade(state, dt);
    state.position = [...fallen.position] as Vec3;
    return this.soldierPose(state, fallen.soldier, -1, fallen.kind);
  }

  private newSoldier(
    soldier: FeedSoldier,
    unit: FeedUnit,
    shots: number,
    generation: number,
  ): SoldierState {
    const state: SoldierState = {
      position: [...soldier.position] as Vec3,
      facing: unit.yaw,
      clip: "idle",
      phase: 0,
      previous: null,
      fadeLeft: 0,
      lastShots: shots,
      firedAt: -Infinity,
      fellAt: null,
      seen: generation,
    };
    this.soldiers.set(soldier.id, state);
    return state;
  }

  /** Move to `clip` (fading from the current one) and advance its phase. */
  private advance(state: SoldierState, clip: string, moved: number, dt: number) {
    if (clip !== state.clip) this.switchTo(state, clip);
    const facts = this.options.clip(state.clip);
    if (facts) {
      // Locomotion with a declared stride advances with ground covered, so
      // feet do not slide; everything else advances with time.
      const step =
        facts.stride_m && (state.clip === "walk" || state.clip === "run")
          ? moved / facts.stride_m
          : dt / facts.duration;
      state.phase = facts.loop ? (state.phase + step) % 1 : Math.min(1, state.phase + step);
    }
    this.fade(state, dt);
  }

  private switchTo(state: SoldierState, clip: string) {
    state.previous = { clip: state.clip, phase: state.phase, weight: 1 };
    state.fadeLeft = GAIT.fade;
    state.clip = clip;
    state.phase = 0;
  }

  private fade(state: SoldierState, dt: number) {
    if (!state.previous) return;
    state.fadeLeft = Math.max(0, state.fadeLeft - dt);
    state.previous.weight = state.fadeLeft / GAIT.fade;
    if (state.previous.weight <= 0) state.previous = null;
  }

  private soldierPose(
    state: SoldierState,
    soldier: number,
    unit: number,
    kind: UnitKindName,
  ): SoldierPose {
    return {
      soldier,
      unit,
      kind,
      position: state.position,
      facing: state.facing,
      clip: state.clip,
      phase: state.phase,
      blend: state.previous ? { ...state.previous } : null,
    };
  }

  private vehicle(unit: FeedUnit, generation: number): VehiclePose {
    let state = this.vehicles.get(unit.id);
    if (!state) {
      state = {
        position: [...unit.position] as Vec3,
        yaw: unit.yaw,
        travelL: 0,
        travelR: 0,
        seen: generation,
      };
      this.vehicles.set(unit.id, state);
    }
    state.seen = generation;
    // Ground covered along the hull, plus each side's share of the turn.
    const forward =
      (unit.position[0] - state.position[0]) * Math.cos(unit.yaw) +
      (unit.position[1] - state.position[1]) * Math.sin(unit.yaw);
    const turned = deltaAngle(state.yaw, unit.yaw);
    const half = this.options.halfTrack[unit.kind] ?? 0;
    state.travelL += forward - turned * half;
    state.travelR += forward + turned * half;
    state.position = [...unit.position] as Vec3;
    state.yaw = unit.yaw;

    const articulation: Articulation = {
      ...REST_ARTICULATION,
      travel_l: state.travelL,
      travel_r: state.travelR,
      deploy: unit.deployment ?? 0,
    };
    const roles = this.options.mounts[unit.kind] ?? [];
    let turretBearing = unit.yaw;
    unit.mounts.forEach((mount, i) => {
      if (roles[i] === "gun") {
        turretBearing = mount.bearing;
        articulation.turret_yaw = deltaAngle(unit.yaw, mount.bearing);
        articulation.gun_pitch = clamp(mount.elevation, PITCH_LIMITS.gun[0], PITCH_LIMITS.gun[1]);
      }
    });
    unit.mounts.forEach((mount, i) => {
      if (roles[i] === "hmg") {
        articulation.hmg_yaw = deltaAngle(turretBearing, mount.bearing);
        articulation.hmg_pitch = clamp(mount.elevation, PITCH_LIMITS.hmg[0], PITCH_LIMITS.hmg[1]);
      }
    });
    return {
      unit: unit.id,
      kind: unit.kind,
      position: state.position,
      yaw: unit.yaw,
      articulation,
    };
  }
}
