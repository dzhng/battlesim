// Brew-ups a side watched, from its publications: a hull it saw that is gone,
// and a wreck, where it stood, soon after. The simulation leaves a destroyed
// vehicle's wreck on its own place and heading the tick it dies
// (`sim::Battle::consequences`); the side loses the hull at once and learns
// the wreck within a few ticks, so a side that saw the hull alive within
// `window` ticks of learning its wreck saw it die. A wreck found later,
// scouted onto, was no one's to watch: it is simply there, burning as the
// effects have it.
//
// Presentation only: the effects blow the hull up (fireballs out of the
// turret ring, and sparks and dust where its turret lands) and, where its
// wreck is cut into pieces, the battle jolts the hull and throws the turret
// (`effects/cookOff.ts`), from the tick the wreck appeared.
import { mat4, type Mat4, type Vec3 } from "math";
import {
  hullMotion,
  turretMotion,
  type CookOffFeel,
} from "@packages/battle-renderer/src/effects/cookOff";
import type { EffectCookOff } from "@packages/battle-renderer/src/effects/effectFrame";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import type { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { WRECK_PIECES } from "@packages/scene-assets/src/scenery";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { KnownPropView, ObservationView } from "@web/battle/sim/observation";

/** A hull the side watched brew up: its wreck as the side knows it, and the
 *  publication tick that first showed it. */
export interface CookOff {
  /** The wreck's known prop id. */
  prop: number;
  kind: string;
  center: readonly [number, number];
  yaw: number;
  half: readonly [number, number, number];
  baseZ: number;
  tick: number;
}

interface Hull {
  wreck: string;
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
                  wreck: hull.wreck,
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
      if (!first && !p.destroyed && this.watched(p))
        out.push({
          prop: p.id,
          kind: p.kind,
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

  /** Whether a hull the side lost from sight lately stood where wreck `p` lies. */
  private watched(p: KnownPropView): boolean {
    return [...this.hulls.values()].some(
      (h) => h.wreck === p.kind && Math.hypot(p.center[0] - h.x, p.center[1] - h.y) <= h.reach,
    );
  }
}

/** A cook-off whose wreck is cut into pieces, as the battle draws it while
 *  they move: the wreck as fitted, and where its turret piece lies in it. */
export interface Flight {
  cookOff: CookOff;
  wreck: ModelInstance;
  /** The turret piece's centre in the wreck's own frame. */
  lies: Vec3;
  /** How high its underside lies on the deck, in the same frame. */
  underside: number;
  /** The presentation second of the killing hit (its tick's start). */
  hitAt: number;
}

/** `c` as the battle draws it, its wreck fitted by `fit` from `installed`;
 *  null where its wreck has no pieces to move. */
export function flightOf(
  c: CookOff,
  fit: PropAppearances,
  installed: InstalledAppearances,
  tickHz: number,
): Flight | null {
  const wreck = fit.fit(
    { kind: c.kind, center: c.center, yaw: c.yaw, half: c.half, baseZ: c.baseZ },
    [],
  )[0];
  const bundle = wreck && installed.appearances.get(wreck.appearance)?.bundle;
  const states = bundle?.kind === "static" ? bundle.states : [];
  const turret = states.find((s) => s.name === WRECK_PIECES.turret);
  if (!turret || !states.some((s) => s.name === WRECK_PIECES.hull)) return null;
  const { min, max } = turret.bounds;
  return {
    cookOff: c,
    wreck,
    lies: [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2],
    underside: min[2],
    hitAt: (c.tick - 1) / tickHz,
  };
}

/** `c` as the effects take it: its hull, and where its turret lands, if its
 *  wreck throws one (`flight`). */
export function effectCookOff(c: CookOff, flight: Flight | null): EffectCookOff {
  return {
    center: [c.center[0], c.center[1], c.baseZ],
    height: 2 * c.half[2],
    landing:
      flight && placedPoint(flight.wreck, [flight.lies[0], flight.lies[1], flight.underside]),
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

/** `f`'s pieces at presentation second `clock`: its hull, jolted
 *  (`hullMotion`), and its turret, thrown (`turretMotion`). */
export function piecesOf(f: Flight, feel: CookOffFeel, clock: number): ModelInstance[] {
  const age = clock - f.hitAt;
  const seed = f.cookOff.prop;
  const piece = (state: string, motion: Mat4): ModelInstance => ({
    ...f.wreck,
    pose: { kind: "static", state, motion },
  });
  return [
    piece(WRECK_PIECES.hull, hullMotion(mat4.create(), feel, age, seed)),
    piece(WRECK_PIECES.turret, turretMotion(mat4.create(), feel, age, f.lies, seed)),
  ];
}
