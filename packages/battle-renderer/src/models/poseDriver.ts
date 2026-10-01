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
// own objects, valid until the next call. The fallen are the one population
// that can grow into the tens of thousands, so they are reconciled only when
// the feed hands over a new `fallen` list, and `corpsesVersion` says when the
// static list changed. Presentation draws at most `corpses.max` of them: past
// it the oldest corpse sinks into the ground (`fading`, posed every frame)
// and is gone, though the simulation still lists him.

import { clamp, deltaAngle, vec3, type Vec2, type Vec3 } from "math";
import { mulberry32 } from "math/random";
import { easing } from "math/time";
import {
  PITCH_LIMITS,
  REST_ARTICULATION,
  type Articulation,
} from "@packages/scene-assets/src/articulation";
import type { Side } from "@packages/scene-assets/src/schema";
import type { MountRole, UnitCatalog } from "@packages/scene-assets/src/units";
import { sideKey } from "../sideKey";

export type Posture = "stand" | "kneel" | "prone";

export interface FeedSoldier {
  id: number;
  /** His slot in his squad type: which soldier kind he is (his appearance). */
  slot: number;
  position: Vec3;
  /** The soldier's own posture, when the simulation publishes one. */
  posture?: Posture;
  /** The rounds he has launched so far (the lab's feed counts them from the
   *  launches the flashes and sounds read); a rise is his shot. The count
   *  holds across every frame of a publication, so a shot is timed once. A
   *  feed that names shooters sets it on every soldier; one that leaves it
   *  absent names no one, and a rise of his squad's shot counter is then a
   *  shot by the whole squad. */
  shots?: number;
  /** Where his body stands while he leans out past his cover's edge to
   *  fire; null or absent while he is tucked in at `position`. */
  lean?: Vec2 | null;
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
  /** Its unit type's catalog id. */
  kind: string;
  side: Side;
  position: Vec3;
  /** Hull heading, radians counter-clockwise from +X. */
  yaw: number;
  soldiers: FeedSoldier[];
  mounts: FeedMount[];
  /** Deployment progress, null for units that do not deploy. */
  deployment: number | null;
  /** The squad is pinned, by the sim's published tier: its soldiers with
   *  no posture of their own go prone (false where the side cannot know it). */
  pinned: boolean;
}

export interface FeedFallen {
  soldier: number;
  position: Vec3;
  /** The published heading of the fallen (his squad's, when he fell). */
  yaw: number;
  /** The unit type of the squad he fought in and his slot in it, for his appearance. */
  kind: string;
  slot: number;
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
  kind: string;
  slot: number;
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
  kind: string;
  side: Side;
  position: Vec3;
  yaw: number;
  articulation: Articulation;
}

/** A fallen soldier whose death has played out (or was never seen): drawn
 *  as a static mesh, never skinned. */
export interface CorpsePose {
  soldier: number;
  kind: string;
  slot: number;
  side: Side;
  position: Vec3;
  yaw: number;
}

/** A corpse pushed out of the static list by the cap, sinking into the
 *  ground until its fade has run (`PoseFeel.corpses`). */
export interface FadingCorpse {
  corpse: CorpsePose;
  /** How far below where he lay he is drawn, metres. */
  sink: number;
  /** When he left the static list, simulation seconds. */
  since: number;
}

export interface PoseFrame {
  soldiers: SoldierPose[];
  vehicles: VehiclePose[];
  /** The static corpses, oldest first, at most `corpses.max`. */
  corpses: CorpsePose[];
  /** Rises whenever `corpses` changed. */
  corpsesVersion: number;
  /** Corpses sinking away, posed every frame; none once their fade has run. */
  fading: FadingCorpse[];
}

/** What the driver needs to know about the clips it picks. */
export interface ClipFacts {
  duration: number;
  loop: boolean;
  stride_m: number | null;
}

/** `presentation.pose`: how poses move between what the simulation
 *  publishes. Presentation feel, not rules. */
export interface PoseFeel {
  /** Gait thresholds and timings. */
  gait: {
    /** Speed from which a soldier walks, and from which he runs, m/s. */
    walk_mps: number;
    run_mps: number;
    /** Crossfade between clips, seconds. */
    fade_s: number;
    /** Speed below which facing does not follow velocity, m/s. */
    facing_mps: number;
    /** Turn rate toward a new facing, radians per second. */
    turn_rad_s: number;
  };
  /** How a squad stands at rest (`restManner`). */
  rest: {
    /** How far a man's gaze strays from the squad's aim, ± radians. */
    turn_rad: number;
    /** The range of his idle's tempo. */
    tempo: [number, number];
    /** One man in this many stands watching, weapon up. */
    watch_every: number;
    /** Seconds after the squad's last shot before the gaze starts to stray,
     *  and over which it strays fully. */
    settle_s: [number, number];
  };
  /** How a soldier slides out to his lean point and back: seconds out,
   *  a quick step, and seconds back in, slower. */
  lean: { out_s: number; back_s: number };
  /** How a vehicle's mounts move between shots. */
  mount: {
    /** A mount's published elevation is its last round's, so it changes only
     *  on a shot; the gun eases to it at up to this rate, radians per second. */
    gun_elevation_rad_s: number;
    hmg_elevation_rad_s: number;
    /** How far the gun runs back on a shot, metres, and how long it takes to
     *  run out to battery again, seconds. */
    recoil_m: number;
    recoil_return_s: number;
  };
  /** Each vehicle type's running-gear half gauge, as a share of its hull's
   *  half width; 1 when absent. */
  gauge: Partial<Record<string, number>>;
  /** How many of the fallen lie drawn at once. Past `max` the oldest sinks
   *  `sink_m` into the ground over `fade_s` seconds, easing in, and is then
   *  gone for good. A presentation cap: the simulation keeps every one. */
  corpses: { max: number; fade_s: number; sink_m: number };
}

/** `presentation.pose`, checked: every rate and time positive, running
 *  faster than walking, ranges ordered, and gauges within the hull. */
export function validatePoseFeel(p: PoseFeel): PoseFeel {
  const fail = (path: string, why: string): never => {
    throw new Error(`presentation.pose.${path}: ${why}, got ${JSON.stringify(p)}`);
  };
  const positive = (path: string, v: number) => {
    if (!(v > 0)) fail(path, "must be positive");
  };
  const { gait, rest, lean, mount, gauge, corpses } = p;
  positive("gait.walk_mps", gait.walk_mps);
  if (!(gait.run_mps > gait.walk_mps)) fail("gait.run_mps", "must exceed walk_mps");
  positive("gait.fade_s", gait.fade_s);
  if (!(gait.facing_mps >= 0)) fail("gait.facing_mps", "must be ≥ 0");
  positive("gait.turn_rad_s", gait.turn_rad_s);
  if (!(rest.turn_rad >= 0)) fail("rest.turn_rad", "must be ≥ 0");
  if (!(rest.tempo[0] > 0 && rest.tempo[1] >= rest.tempo[0]))
    fail("rest.tempo", "must be a positive [low, high]");
  // The watcher is the man whose id leaves 2 over `watch_every`.
  if (!(Number.isInteger(rest.watch_every) && rest.watch_every >= 3))
    fail("rest.watch_every", "must be an integer ≥ 3");
  if (!(rest.settle_s[0] >= 0 && rest.settle_s[1] > 0))
    fail("rest.settle_s", "must be [delay ≥ 0, over > 0]");
  positive("lean.out_s", lean.out_s);
  positive("lean.back_s", lean.back_s);
  positive("mount.gun_elevation_rad_s", mount.gun_elevation_rad_s);
  positive("mount.hmg_elevation_rad_s", mount.hmg_elevation_rad_s);
  if (!(mount.recoil_m >= 0)) fail("mount.recoil_m", "must be ≥ 0");
  positive("mount.recoil_return_s", mount.recoil_return_s);
  for (const [kind, share] of Object.entries(gauge))
    if (!(share! > 0 && share! <= 1)) fail(`gauge.${kind}`, "must be in (0, 1]");
  if (!(Number.isInteger(corpses.max) && corpses.max >= 1))
    fail("corpses.max", "must be an integer ≥ 1");
  positive("corpses.fade_s", corpses.fade_s);
  if (!(corpses.sink_m >= 0)) fail("corpses.sink_m", "must be ≥ 0");
  return p;
}

export interface PoseDriverOptions {
  /** The unit catalog: which units are vehicles, and their hulls. */
  units: UnitCatalog;
  /** How a type's model draws each of its mounts, in mount order: the rig
   *  its appearance declares, or by hand (`AppearanceCatalog.mountRoles`). */
  mounts: (kind: string) => readonly MountRole[];
  /** A type's clip durations and strides, for phase; null for a clip its rig lacks. */
  clip: (kind: string, name: string) => ClipFacts | null;
  /** `presentation.pose`, validated (`validatePoseFeel`). */
  feel: PoseFeel;
  /** Seconds a soldier stays in his firing pose after a shot: the rules'
   *  `cover.lean_hold_s`, the time the simulation keeps him out on a lean. */
  leanHold: number;
}

/** Where a soldier starts a looping clip: his own offset (the golden ratio
 *  over his id), so a squad that starts walking, kneels or breathes together
 *  never moves in lockstep. A one-shot clip (death) starts at its start. */
export function loopStart(soldier: number): number {
  return (soldier * 0.6180339887498949) % 1;
}

/** How a soldier stands at rest, his own for life (from his id): how far his
 *  gaze strays from the squad's aim (radians, within ±`rest.turn_rad`), how
 *  fast his idle plays (within `rest.tempo`), and whether he stands watching,
 *  his weapon up (`stand_aim`, one man in `rest.watch_every`), rather than at
 *  ease. A squad at rest scans different ways, in different stances, out of
 *  step, never a row of copies; while his squad is shooting every man faces
 *  the aim, and the gaze strays again only once the squad has been quiet for
 *  `rest.settle_s`. */
export function restManner(
  soldier: number,
  rest: PoseFeel["rest"],
): { turn: number; tempo: number; watch: boolean } {
  const state = mulberry32.create(Math.imul(soldier + 1, 0x9e3779b1) >>> 0);
  const turn = (mulberry32.sample(state) * 2 - 1) * rest.turn_rad;
  const tempo = rest.tempo[0] + mulberry32.sample(state) * (rest.tempo[1] - rest.tempo[0]);
  // every `watch_every`-th man by id, so each squad mixes its stances
  const watch = soldier % rest.watch_every === 2;
  return { turn, tempo, watch };
}

interface SoldierState {
  pose: SoldierPose;
  /** Where the simulation last put him (tucked in): his gait reads this,
   *  never the drawn position, so a lean is never a walk. */
  at: Vec3;
  /** How far out on his lean he is drawn, 0 tucked in to 1 out, and the
   *  lean point he slides to (kept while he eases back from it). */
  leanT: number;
  leanAt: Vec3;
  /** His `restManner`, and when his squad last fired. */
  turn: number;
  tempo: number;
  watch: boolean;
  alertAt: number;
  /** The blend `pose.blend` points at while a fade runs. */
  fading: ClipBlend;
  fadeLeft: number;
  /** His squad's shot counter, and his own count, as last seen. */
  lastShots: number;
  ownShots: number;
  firedAt: number;
  /** When he fell, once the feed lists him among the fallen. */
  fellAt: number | null;
  seen: number;
}

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
export function recoilAt(since: number, mount: PoseFeel["mount"]): number {
  if (!(since >= 0) || since >= mount.recoil_return_s) return 0;
  const left = 1 - since / mount.recoil_return_s;
  return mount.recoil_m * left * left;
}

/** Keep the entries of `list` that pass `test`, in order, in place. */
function keepWhere<T>(list: T[], test: (item: T) => boolean) {
  let kept = 0;
  for (const item of list) if (test(item)) list[kept++] = item;
  list.length = kept;
}

export class PoseDriver {
  private readonly soldiers = new Map<number, SoldierState>();
  /** By `sideKey`: an identified enemy's handle can equal an own unit's id. */
  private readonly vehicles = new Map<number, VehicleState>();
  /** Soldiers playing their death, by id (a subset of `soldiers`). */
  private readonly dying = new Set<number>();
  /** The static corpses in the order they came to lie: oldest first. */
  private readonly corpseMap = new Map<number, CorpsePose>();
  /** Corpses laid since the list was last published: never drawn, so the
   *  cap drops them without a fade. */
  private readonly unshown = new Set<number>();
  /** Soldiers the cap has taken away, fading or gone: never laid again. */
  private readonly gone = new Set<number>();
  private lastFallen: readonly FeedFallen[] | null = null;
  private time: number | null = null;
  private generation = 0;
  private readonly out: PoseFrame = {
    soldiers: [],
    vehicles: [],
    corpses: [],
    corpsesVersion: 0,
    fading: [],
  };

  constructor(private readonly options: PoseDriverOptions) {}

  /** Half the distance between a vehicle type's tracks or wheel rows: its
   *  hull's half width by its `presentation.pose.gauge` share. */
  private halfTrack(kind: string): number {
    const hull = this.options.units.hull(kind);
    return hull ? hull.half_extents_m[1] * (this.options.feel.gauge[kind] ?? 1) : 0;
  }

  /** Forget everything: the next frame starts fresh, as if first seen. */
  reset() {
    this.soldiers.clear();
    this.vehicles.clear();
    this.dying.clear();
    this.corpseMap.clear();
    this.unshown.clear();
    this.gone.clear();
    this.lastFallen = null;
    this.time = null;
    this.out.corpses.length = 0;
    this.out.corpsesVersion++;
    this.out.fading.length = 0;
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
      if (this.options.units.hull(unit.kind))
        out.vehicles.push(this.vehicle(unit, frame.time, dt, generation));
      else this.squad(unit, frame.time, dt, generation);
    }
    let relisted = false;
    if (frame.fallen !== this.lastFallen) {
      this.lastFallen = frame.fallen;
      relisted = this.reconcileFallen(frame.fallen, frame.time);
    }
    if (this.advanceDying(frame.time, dt, generation) || relisted) this.publishCorpses(frame.time);
    this.advanceFading(frame.time);
    for (const [id, s] of this.soldiers)
      if (s.seen !== generation) {
        this.soldiers.delete(id);
        this.dying.delete(id);
      }
    for (const [id, v] of this.vehicles) if (v.seen !== generation) this.vehicles.delete(id);
    return out;
  }

  private squad(unit: FeedUnit, time: number, dt: number, generation: number) {
    const roles = this.options.mounts(unit.kind);
    const { gait, rest, lean: slide } = this.options.feel;
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
    for (const soldier of unit.soldiers) {
      const state =
        this.soldiers.get(soldier.id) ?? this.newSoldier(soldier, unit, shots, generation);
      state.seen = generation;
      const pose = state.pose;
      const dx = soldier.position[0] - state.at[0];
      const dy = soldier.position[1] - state.at[1];
      const moved = Math.hypot(dx, dy);
      const speed = dt > 0 ? moved / dt : 0;
      // He fired when his own count rose; a feed that names no one leaves it
      // to a rise of the squad's counter, a shot by the whole squad.
      const own = soldier.shots;
      if (own === undefined ? shots > state.lastShots : own > state.ownShots) state.firedAt = time;
      if (shots > state.lastShots) state.alertAt = time;
      state.lastShots = shots;
      state.ownShots = own ?? 0;
      const firing = time - state.firedAt < this.options.leanHold;

      // Facing: his own velocity, else the weapon's aim (or the unit's heading),
      // strayed by his own manner once the squad has settled.
      const settled = clamp((time - state.alertAt - rest.settle_s[0]) / rest.settle_s[1], 0, 1);
      const target = speed > gait.facing_mps ? Math.atan2(dy, dx) : aim + state.turn * settled;
      const turn = deltaAngle(pose.facing, target);
      const step = gait.turn_rad_s * dt;
      pose.facing += Math.abs(turn) <= step ? turn : Math.sign(turn) * step;

      // Out on his lean he kneels to fire, pinned or not: the film's man
      // pops out from behind the tree and drops back.
      const posture =
        soldier.posture ??
        (soldier.lean ? "kneel" : unit.pinned ? "prone" : firing ? "kneel" : "stand");

      const clip =
        posture === "prone"
          ? "prone_pinned"
          : posture === "kneel" && speed < gait.walk_mps
            ? "kneel_fire"
            : speed >= gait.run_mps
              ? "run"
              : speed >= gait.walk_mps
                ? "walk"
                : state.watch
                  ? "stand_aim"
                  : "idle";
      this.advance(state, clip, moved, dt);
      vec3.copy(state.at, soldier.position);
      // Out on his lean: slide to the lean point; tucked in: ease back.
      const lean = soldier.lean ?? null;
      if (lean) vec3.set(state.leanAt, lean[0], lean[1], soldier.position[2]);
      state.leanT = lean
        ? Math.min(1, state.leanT + dt / slide.out_s)
        : Math.max(0, state.leanT - dt / slide.back_s);
      vec3.lerp(pose.position, soldier.position, state.leanAt, easing.sineInOut(state.leanT));
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
    const { turn, tempo, watch } = restManner(soldier.id, this.options.feel.rest);
    const state: SoldierState = {
      pose: {
        soldier: soldier.id,
        unit: unit.id,
        kind: unit.kind,
        slot: soldier.slot,
        side: unit.side,
        position: vec3.clone(soldier.position),
        facing: unit.yaw + turn,
        clip: "idle",
        phase: loopStart(soldier.id),
        blend: null,
      },
      at: vec3.clone(soldier.position),
      leanT: 0,
      leanAt: vec3.clone(soldier.position),
      turn,
      tempo,
      watch,
      alertAt: -Infinity,
      fading: { clip: "idle", phase: 0, weight: 0 },
      fadeLeft: 0,
      // Rounds fired before he was first seen are not his shot.
      lastShots: shots,
      ownShots: soldier.shots ?? 0,
      firedAt: -Infinity,
      fellAt: null,
      seen: generation,
    };
    this.soldiers.set(soldier.id, state);
    return state;
  }

  /** A new `fallen` list: soldiers seen alive start their death; the rest
   *  (and anyone first seen already down) lie as corpses at once. Whether the
   *  static corpses changed. */
  private reconcileFallen(fallen: readonly FeedFallen[], time: number): boolean {
    let changed = false;
    const listed = new Set<number>();
    const fading = new Map(this.out.fading.map((f) => [f.corpse.soldier, f.corpse]));
    for (const f of fallen) {
      listed.add(f.soldier);
      const corpse = this.corpseMap.get(f.soldier);
      if (corpse) {
        if (corpse.position.some((v, i) => v !== f.position[i])) {
          vec3.copy(corpse.position, f.position);
          changed = true;
        }
        continue;
      }
      if (this.gone.has(f.soldier)) {
        const fadingCorpse = fading.get(f.soldier);
        if (fadingCorpse) vec3.copy(fadingCorpse.position, f.position);
        continue;
      }
      const state = this.soldiers.get(f.soldier);
      if (state && state.fellAt === null) {
        state.fellAt = time;
        vec3.copy(state.pose.position, f.position);
        state.pose.unit = -1;
        this.switchTo(state, "death");
        this.dying.add(f.soldier);
      } else if (state) {
        // Authority can remove a floor while the same death is still playing.
        vec3.copy(state.pose.position, f.position);
      } else {
        this.lay({
          soldier: f.soldier,
          kind: f.kind,
          slot: f.slot,
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
        this.unshown.delete(id);
        changed = true;
      }
    for (const id of this.gone) if (!listed.has(id)) this.gone.delete(id);
    keepWhere(this.out.fading, (f) => listed.has(f.corpse.soldier));
    for (const id of this.dying)
      if (!listed.has(id)) {
        this.dying.delete(id);
        this.soldiers.delete(id);
      }
    return changed;
  }

  /** Play each death on; a finished one becomes a static corpse. Whether
   *  any did. */
  private advanceDying(time: number, dt: number, generation: number): boolean {
    let laid = false;
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
        this.lay({
          soldier: id,
          kind: pose.kind,
          slot: pose.slot,
          side: pose.side,
          position: vec3.clone(pose.position),
          yaw: pose.facing,
        });
        laid = true;
        continue;
      }
      this.out.soldiers.push(pose);
    }
    return laid;
  }

  /** A new static corpse, the newest. */
  private lay(corpse: CorpsePose) {
    this.corpseMap.set(corpse.soldier, corpse);
    this.unshown.add(corpse.soldier);
  }

  /** Hold the static list to `corpses.max`, oldest out first: one that was
   *  drawn starts to sink; one never drawn just goes. Then publish it. */
  private publishCorpses(time: number) {
    const excess = this.corpseMap.size - this.options.feel.corpses.max;
    if (excess > 0) {
      let n = 0;
      for (const [id, corpse] of this.corpseMap) {
        if (n++ === excess) break;
        this.corpseMap.delete(id);
        this.gone.add(id);
        if (!this.unshown.has(id)) this.out.fading.push({ corpse, sink: 0, since: time });
      }
    }
    this.unshown.clear();
    const corpses = this.out.corpses;
    corpses.length = 0;
    for (const c of this.corpseMap.values()) corpses.push(c);
    this.out.corpsesVersion++;
  }

  /** Sink each fading corpse, easing in; drop those whose fade has run. */
  private advanceFading(time: number) {
    const { fade_s, sink_m } = this.options.feel.corpses;
    keepWhere(this.out.fading, (f) => {
      const t = (time - f.since) / fade_s;
      if (t >= 1) return false;
      f.sink = sink_m * easing.sineIn(t);
      return true;
    });
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
    state.fadeLeft = this.options.feel.gait.fade_s;
    pose.clip = clip;
    pose.phase = this.options.clip(pose.kind, clip)?.loop ? loopStart(pose.soldier) : 0;
  }

  private fade(state: SoldierState, dt: number) {
    const pose = state.pose;
    if (!pose.blend) return;
    state.fadeLeft = Math.max(0, state.fadeLeft - dt);
    state.fading.weight = state.fadeLeft / this.options.feel.gait.fade_s;
    if (state.fading.weight <= 0) pose.blend = null;
  }

  private vehicle(unit: FeedUnit, time: number, dt: number, generation: number): VehiclePose {
    const roles = this.options.mounts(unit.kind);
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
    const key = sideKey(unit.id, unit.side, "blue");
    let state = this.vehicles.get(key);
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
      this.vehicles.set(key, state);
    }
    state.seen = generation;
    const pose = state.pose;
    const a = pose.articulation;
    // Ground covered along the hull, plus each side's share of the turn.
    const forward =
      (unit.position[0] - pose.position[0]) * Math.cos(unit.yaw) +
      (unit.position[1] - pose.position[1]) * Math.sin(unit.yaw);
    const turned = deltaAngle(pose.yaw, unit.yaw);
    const half = this.halfTrack(unit.kind);
    a.travel_l += forward - turned * half;
    a.travel_r += forward + turned * half;
    vec3.copy(pose.position, unit.position);
    pose.yaw = unit.yaw;
    a.deploy = unit.deployment ?? 0;

    // The turret on the cannon's bearing, the HMG relative to what its
    // catalog row rides (`on`: a mount, else the hull); elevations eased
    // (they change only on a shot); a new cannon round recoils the gun.
    const rows = this.options.units.type(unit.kind).mounts;
    const on = hmg >= 0 ? rows[hmg]?.on : null;
    const carrier = on ? unit.mounts[rows.findIndex((m) => m.name === on)] : undefined;
    const carrierBearing = carrier ? carrier.bearing : unit.yaw;
    a.turret_yaw = gunMount ? deltaAngle(unit.yaw, gunMount.bearing) : 0;
    const feel = this.options.feel.mount;
    a.gun_pitch = approach(a.gun_pitch, gunTarget, feel.gun_elevation_rad_s * dt);
    a.hmg_yaw = hmgMount ? deltaAngle(carrierBearing, hmgMount.bearing) : 0;
    a.hmg_pitch = approach(a.hmg_pitch, hmgTarget, feel.hmg_elevation_rad_s * dt);
    if (gunMount && gunMount.shots > state.gunShots) state.firedAt = time;
    if (gunMount) state.gunShots = gunMount.shots;
    a.recoil = recoilAt(time - state.firedAt, feel);
    return pose;
  }
}
