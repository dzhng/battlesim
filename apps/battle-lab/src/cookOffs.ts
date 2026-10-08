// Brew-ups a side watched, from its publications: a hull it saw that is gone,
// and a wreck near where it stood. The simulation lays a destroyed vehicle's
// wreck where it rolls to a stop, on its heading, the tick it dies
// (`sim::Battle::consequences`), and a side that knew the hull learns the
// wreck that tick, so a side that saw the hull alive within `window` ticks
// of learning its wreck saw it die. A wreck found later,
// scouted onto, was no one's to watch: it is simply there, burning as the
// effects have it.
//
// Presentation only: the effects blow the hull up (fireballs out of the
// turret ring, and sparks and dust where its turret lands) and, where its
// wreck is cut into pieces, the battle jolts the hull and throws the turret;
// other wrecks jolt whole (`effects/cookOff.ts`), from the tick the wreck
// appeared. Until the ammunition goes the hull is drawn whole, as the side
// last saw it, so the fire turns the live vehicle into the wreck. What the
// blast throws clear (the wreck's `debris` state) lies round it for a while
// and then sinks away (`debrisModel`); a wreck found later has none.
import { mat4, type Mat4, type Vec3 } from "math";
import {
  debrisSink,
  hullMotion,
  turretMotion,
  type CookOffFeel,
} from "@packages/battle-renderer/src/effects/cookOff";
import type { EffectCookOff } from "@packages/battle-renderer/src/effects/effectFrame";
import type {
  ModelInstance,
  ResolveAppearance,
} from "@packages/battle-renderer/src/models/modelInstances";
import type { VehiclePose } from "@packages/battle-renderer/src/models/poseDriver";
import type { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { Bounds } from "@packages/scene-assets/src/schema";
import { SCENERY_KINDS, WRECK_PIECES } from "@packages/scene-assets/src/scenery";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { KnownPropView, ObservationView } from "@web/battle/sim/observation";

/** A hull the side watched brew up: its wreck as the side knows it, and the
 *  publication tick that first showed it. */
export interface CookOff {
  /** The wreck's known prop id. */
  prop: number;
  kind: string;
  /** The unit type it is the wreck of. */
  wreckOf: string;
  center: readonly [number, number];
  yaw: number;
  half: readonly [number, number, number];
  baseZ: number;
  tick: number;
}

interface Hull {
  /** Its unit type: the wreck it leaves names it (`wreckOf`). */
  kind: string;
  /** The publication it was last seen in. */
  tick: number;
  x: number;
  y: number;
  /** How far its wreck may lie from where it was seen: its half length. */
  reach: number;
}

export class CookOffWatch {
  /** Every hull seen within `window` ticks, where it was last seen, by
   *  side-qualified id. */
  private hulls = new Map<string, Hull>();
  private known = new Set<number>();
  private lastTick = -1;

  constructor(
    private readonly units: UnitCatalog,
    private readonly window: number,
  ) {}

  /** Take the side's next publication: the hulls it watched brew up since
   *  the last one. An earlier tick is a new battle and starts over. */
  note(o: ObservationView): CookOff[] {
    if (o.tick < this.lastTick) {
      this.hulls.clear();
      this.known.clear();
      this.lastTick = -1;
    }
    const first = this.lastTick < 0;
    const seen = (
      [
        ["own", o.own],
        ["enemy", o.identified],
      ] as const
    ).flatMap(([who, us]) =>
      us.flatMap((u) => {
        const hull = this.units.hull(u.kind);
        return hull
          ? [
              [
                `${who}:${u.id}`,
                {
                  kind: u.kind,
                  tick: o.tick,
                  x: u.position[0],
                  y: u.position[1],
                  reach: hull.half_extents_m[0],
                },
              ] as const,
            ]
          : [];
      }),
    );
    // Lost from sight this publication, within the window: those it could
    // have watched die.
    const live = new Set<string>(seen.map(([key]) => key));
    for (const [key, h] of this.hulls)
      if (live.has(key) || o.tick - h.tick > this.window) this.hulls.delete(key);
    const out: CookOff[] = [];
    for (const p of o.knownProps) {
      if (this.known.has(p.id)) continue;
      this.known.add(p.id);
      if (!first && !p.destroyed && p.wreckOf !== null && this.watched(p))
        out.push({
          prop: p.id,
          kind: p.kind,
          wreckOf: p.wreckOf,
          center: [p.center[0], p.center[1]],
          yaw: p.yaw,
          half: [p.half[0], p.half[1], p.half[2]],
          baseZ: p.baseZ,
          tick: o.tick,
        });
    }
    for (const [key, h] of seen) this.hulls.set(key, h);
    this.lastTick = o.tick;
    return out;
  }

  /** Whether a hull the side lost from sight lately, of the unit type wreck
   *  `p` was, stood where it lies. */
  private watched(p: KnownPropView): boolean {
    return [...this.hulls.values()].some(
      (h) =>
        h.kind === p.wreckOf &&
        Math.hypot(p.center[0] - h.x, p.center[1] - h.y) <= h.reach + ROLL_REACH_M,
    );
  }
}

/** A moving wreck, either whole or cut into a hull and thrown turret. */
export interface CookOffTransition {
  cookOff: CookOff;
  wreck: ModelInstance;
  /** Resting bounds, used to keep a whole wreck above its ground plane. */
  bounds: Bounds;
  /** The turret piece's centre in the wreck's own frame, or null for a whole wreck. */
  lies: Vec3 | null;
  /** How high its underside lies on the deck, in the same frame. */
  underside: number;
  /** How tall its thrown debris lies, metres, or null for a wreck that
   *  throws none. */
  debrisTop: number | null;
  /** The presentation second of the killing hit (its tick's start). */
  hitAt: number;
}

/** Fit any watched vehicle wreck for its death transition: its own unit's
 *  wreck. Only a wreck with both authored pieces throws a turret (a tank's);
 *  others jolt whole (a wheeled carrier's, which has no turret to throw). */
export function transitionOf(
  c: CookOff,
  fit: PropAppearances,
  installed: InstalledAppearances,
  tickHz: number,
): CookOffTransition | null {
  const wreck = fit.fit(
    {
      kind: c.kind,
      center: c.center,
      yaw: c.yaw,
      half: c.half,
      baseZ: c.baseZ,
      wreckOf: c.wreckOf,
    },
    [],
  )[0];
  const bundle = wreck && installed.appearances.get(wreck.appearance)?.bundle;
  const states = bundle?.kind === "static" ? bundle.states : [];
  const turret = states.find((s) => s.name === WRECK_PIECES.turret);
  const whole = states.find((s) => s.name === "default");
  if (!wreck || !whole) return null;
  const moving = turret && states.some((s) => s.name === WRECK_PIECES.hull) ? turret : null;
  const debris = states.find((s) => s.name === SCENERY_KINDS.wreck.debris?.state);
  const min = moving?.bounds.min;
  const max = moving?.bounds.max;
  return {
    cookOff: c,
    wreck,
    bounds: whole.bounds,
    lies: min && max ? [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2] : null,
    underside: min?.[2] ?? 0,
    debrisTop: debris ? debris.bounds.max[2] : null,
    hitAt: (c.tick - 1) / tickHz,
  };
}

/** `c` as the effects take it: its hull, and where its turret lands, if its
 *  wreck throws one (`transition`). */
export function effectCookOff(c: CookOff, transition: CookOffTransition | null): EffectCookOff {
  return {
    center: [c.center[0], c.center[1], c.baseZ],
    height: 2 * c.half[2],
    landing: transition?.lies
      ? placedPoint(transition.wreck, [
          transition.lies[0],
          transition.lies[1],
          transition.underside,
        ])
      : null,
  };
}

/** `p`, in `model`'s own frame, where `model` places it in the world. */
function placedPoint(model: ModelInstance, p: Vec3): Vec3 {
  const [sx, sy, sz] = model.scale ?? [1, 1, 1];
  const [x, y] = [p[0] * sx, p[1] * sy];
  const c = Math.cos(model.yaw);
  const s = Math.sin(model.yaw);
  return [model.x + x * c - y * s, model.y + x * s + y * c, model.z + p[2] * sz];
}

/** The wreck at `clock`: jolting whole, or as its hull and thrown turret. */
export function wreckModels(
  f: CookOffTransition,
  feel: CookOffFeel,
  clock: number,
): ModelInstance[] {
  const age = clock - f.hitAt;
  const seed = f.cookOff.prop;
  const piece = (state: string, motion: Mat4): ModelInstance => ({
    ...f.wreck,
    pose: { kind: "static", state, motion },
  });
  if (!f.lies) {
    const motion = hullMotion(mat4.create(), feel, age, seed);
    const { min, max } = f.bounds;
    // A tank hull clears the deck; a complete wreck includes wheels and
    // loose panels on it. Raise its lowest rotated bound to the resting plane.
    const low =
      Math.min(motion[2] * min[0], motion[2] * max[0]) +
      Math.min(motion[6] * min[1], motion[6] * max[1]) +
      Math.min(motion[10] * min[2], motion[10] * max[2]) +
      motion[14];
    motion[14] += Math.max(0, min[2] - low);
    return [piece("default", motion)];
  }
  return [
    piece(WRECK_PIECES.hull, hullMotion(mat4.create(), feel, age, seed)),
    piece(WRECK_PIECES.turret, turretMotion(mat4.create(), feel, age, f.lies, seed)),
  ];
}

/** How far below its own top thrown debris sinks before it is gone, in the
 *  wreck's frame (metres, as fitted near enough):
 *  ground that falls away under a piece still hides it. */
const DEBRIS_BURIED_M = 0.15;

/** The debris cook-off `f` threw, at presentation second `clock`: lying
 *  where its wreck lies (not jolting with it) from the blast, then sinking
 *  away (`effects/cookOff.ts` `debrisSink`); null when it is not drawn. */
export function debrisModel(
  f: CookOffTransition,
  feel: CookOffFeel,
  clock: number,
): ModelInstance | null {
  if (f.debrisTop === null) return null;
  const sink = debrisSink(feel, clock - f.hitAt, f.debrisTop + DEBRIS_BURIED_M);
  if (sink === null) return null;
  const motion = mat4.fromTranslation(mat4.create(), [0, 0, -sink]);
  const state = SCENERY_KINDS.wreck.debris!.state;
  return { ...f.wreck, pose: { kind: "static", state, motion } };
}

/** Seconds a vehicle gone from the frame is still remembered as last drawn:
 *  longer than a cook-off waits to blow up. */
const REMEMBERED_S = 2;

/** How far beyond its own half length a wreck may lie from where its hull
 *  was last seen: a vehicle killed at full road speed rolls a few metres on
 *  before it stops (`sim::movement::drive::death_roll`), with room to spare. */
export const ROLL_REACH_M = 6;

/** A hull as the side last saw it, and how hard it brakes, m/s². */
export interface LastHull {
  model: ModelInstance;
  braking: number;
}

/** Each recently drawn vehicle, retained to start a watched death transition.
 *  Records update in place so a warm frame allocates nothing. */
export class LastSeenHulls {
  /** `braking`: a unit type's deceleration as its brakes stop it, m/s². */
  constructor(private readonly braking: (kind: string) => number) {}

  private readonly hulls = new Map<number, { pose: VehiclePose; at: number }>();
  /** The clock of the last frame noted: a hull it did not draw is gone. */
  private clock = -Infinity;

  /** The vehicles the frame at `clock` draws. */
  note(vehicles: readonly VehiclePose[], clock: number): void {
    for (const v of vehicles) {
      const key = v.side === "blue" ? v.unit : -1 - v.unit;
      let h = this.hulls.get(key);
      if (!h) {
        h = {
          pose: { ...v, position: [0, 0, 0], articulation: { ...v.articulation } },
          at: clock,
        };
        this.hulls.set(key, h);
      }
      h.pose.kind = v.kind;
      h.pose.position[0] = v.position[0];
      h.pose.position[1] = v.position[1];
      h.pose.position[2] = v.position[2];
      h.pose.yaw = v.yaw;
      Object.assign(h.pose.articulation, v.articulation);
      h.at = clock;
    }
    for (const [key, h] of this.hulls) if (clock - h.at > REMEMBERED_S) this.hulls.delete(key);
    this.clock = clock;
  }

  /** The hull, gone from the frame lately, nearest where `c`'s wreck lies
   *  (within a roll to a stop), as last drawn; null if none. */
  at(c: CookOff, clock: number, resolve: ResolveAppearance): LastHull | null {
    let best: { pose: VehiclePose; d: number } | null = null;
    for (const { pose, at } of this.hulls.values()) {
      const gone = at < this.clock && clock - at <= REMEMBERED_S;
      const d = Math.hypot(pose.position[0] - c.center[0], pose.position[1] - c.center[1]);
      if (gone && pose.kind === c.wreckOf && d <= c.half[0] + ROLL_REACH_M && (!best || d < best.d))
        best = { pose, d };
    }
    if (best) {
      const { pose } = best;
      const looks = resolve(pose.kind, pose.side, pose.unit, 0);
      if (!looks) return null;
      return {
        model: {
          appearance: looks.appearance,
          tint: looks.tint,
          x: pose.position[0],
          y: pose.position[1],
          z: pose.position[2],
          yaw: pose.yaw,
          pose: { kind: "articulated", articulation: { ...pose.articulation } },
        },
        braking: this.braking(pose.kind),
      };
    }
    return null;
  }
}

/** What cook-off `f` draws at presentation second `clock`: its hull whole,
 *  as last seen (`hull`), until the ammunition goes; then its moving wreck.
 *  A hull killed on the move rolls on to where its wreck lies, slowing as
 *  its stopped running gear slows it, carrying the moving wreck with it. */
export function cookOffModels(
  f: CookOffTransition,
  last: LastHull | null,
  feel: CookOffFeel,
  clock: number,
): ModelInstance[] {
  const t = clock - f.hitAt;
  if (!last) return wreckModels(f, feel, clock);
  const { model: hull, braking } = last;
  // Constant deceleration over `length` metres: done in √(2·length/braking) s.
  const [dx, dy] = [f.wreck.x - hull.x, f.wreck.y - hull.y];
  const length = Math.hypot(dx, dy);
  const done = Math.sqrt((2 * length) / braking);
  const s = t >= done ? 1 : (braking * done * t - (braking * t * t) / 2) / length;
  const [x, y] = [hull.x + dx * s, hull.y + dy * s];
  if (t < feel.delay_s) return [{ ...hull, x, y }];
  const models = wreckModels(f, feel, clock);
  for (const m of models) {
    m.x += x - f.wreck.x;
    m.y += y - f.wreck.y;
  }
  return models;
}
