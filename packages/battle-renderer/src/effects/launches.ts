// Which shots a side's publications show, the one derivation both the
// muzzle flashes (`EffectFrame`) and the gunfire sounds (`battle-audio`)
// read. A mount's shot counter rising is a shot. A hull's mount fires from
// its own muzzle (its `mounts` row, `mountMuzzle.ts`) along its bearing and
// elevation, on the tick its counter rises. A
// squad's rise goes to the soldiers who start new rounds, one round each,
// matched by kind, in the next publication: the simulation flies a round
// first on the tick after the one that fired it, so a squad's rise at tick T
// names rounds whose stretches start at their muzzles at T + 1. A round
// still flying (its stretch starts where last tick's ended) is not a new
// launch. A unit first seen, or seen again, shows no shot until its counter
// rises once more.
import { muzzleOffset } from "@packages/scene-assets/src/mountMuzzle";
import { vec3 } from "math";
import { mulberry32 } from "math/random";
import { hashString } from "@packages/renderer-core/src/math";
import type { EffectPublication, EffectSegment, EffectShooter } from "./effectFrame";

type P3 = readonly [number, number, number] | readonly number[];

/** One shot the publication shows, at the start of its tick. */
export interface Launch {
  /** The shooter's key (`EffectShooter.key`). */
  shooter: number;
  /** The mount that fired: its index in the shooter's mounts. */
  mount: number;
  /** The soldier who fired (a squad's), or null for a hull's mount. */
  soldier: number | null;
  /** Where the round leaves: the hull's muzzle, or the soldier's first path point. */
  x: number;
  y: number;
  z: number;
  /** The unit direction it leaves along. */
  dx: number;
  dy: number;
  dz: number;
  /** The round kind: a weapon row name. */
  kind: string;
  /** Fired from a hull's mount (a vehicle gun), not a soldier's weapon. */
  hull: boolean;
}

const endKey = (p: P3) => `${p[0]},${p[1]},${p[2]}`;
const flightKey = (s: EffectSegment, p: P3) => `${s.kind}:${s.shooter}:${endKey(p)}`;

const _launch_offset = vec3.create();

export class LaunchTracker {
  /** Shot counters by shooter key, as last published. */
  private counters = new Map<number, number[]>();
  /** Where the last publication's still-flying stretches ended. */
  private flying = new Map<string, boolean[]>();
  /** This publication's stretches whose round was chosen as a tracer. */
  readonly tracers = new Set<EffectSegment>();
  /** Squads' rises in the last publication, by shooter key and mount: the
   *  rounds owed to this publication's new stretches. */
  private owed = new Map<number, number[]>();

  reset() {
    this.counters.clear();
    this.flying.clear();
    this.tracers.clear();
    this.owed.clear();
  }

  /** The launches `pub` shows. Call once per publication, in order; `gap`
   *  says ticks were skipped since the last one. */
  note(pub: EffectPublication, gap: boolean, chance?: (kind: string) => number): Launch[] {
    const out: Launch[] = [];
    const starts = new Map<number, EffectSegment[]>();
    for (const s of pub.segments) {
      if (s.shooter === null || s.path.length < 2) continue;
      if (!gap && this.flying.has(flightKey(s, s.path[0]))) continue; // a round still flying
      const list = starts.get(s.shooter);
      if (list) list.push(s);
      else starts.set(s.shooter, [s]);
    }
    const seen = new Set<number>();
    const owed = new Map<number, number[]>();
    for (const u of pub.shooters) {
      seen.add(u.key);
      const before = this.counters.get(u.key);
      this.counters.set(
        u.key,
        u.mounts.map((m) => m.shots),
      );
      // The last publication's rises name this one's new stretches; after a
      // gap they are stale.
      const due = gap ? undefined : this.owed.get(u.key);
      for (let m = 0; m < u.mounts.length; m++) {
        const mount = u.mounts[m];
        const rose = before ? mount.shots - (before[m] ?? mount.shots) : 0; // first seen: no shot
        if (u.half) {
          if (rose > 0) out.push(this.hullLaunch(u, m));
          continue;
        }
        if (rose > 0) {
          const rises = owed.get(u.key) ?? [];
          rises[m] = rose;
          owed.set(u.key, rises);
        }
        let left = due?.[m] ?? 0;
        for (const id of u.members) {
          for (const s of starts.get(id) ?? []) {
            if (left <= 0 || s.kind !== mount.kind) continue;
            left--;
            const [a, b] = [s.path[0], s.path[1]];
            const len = Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2]) || 1;
            out.push({
              shooter: u.key,
              mount: m,
              soldier: id,
              x: a[0],
              y: a[1],
              z: a[2],
              dx: (b[0] - a[0]) / len,
              dy: (b[1] - a[1]) / len,
              dz: (b[2] - a[2]) / len,
              kind: s.kind,
              hull: false,
            });
          }
        }
      }
    }
    this.owed = owed;
    // Units no longer seen start over when they are seen again.
    for (const key of this.counters.keys()) if (!seen.has(key)) this.counters.delete(key);
    const flying = new Map<string, boolean[]>();
    this.tracers.clear();
    for (const [index, s] of pub.segments.entries()) {
      if (s.path.length < 2) continue;
      const probability = chance?.(s.kind) ?? 1;
      // The authority publishes in round order. A queue preserves separate
      // choices when even the same weapon's paths share an exact endpoint.
      const previous = gap ? undefined : this.flying.get(flightKey(s, s.path[0]))?.shift();
      const visible =
        previous ??
        (probability >= 1 ||
          mulberry32.sample(
            mulberry32.create(
              hashString(`${pub.tick}:${index}:${s.kind}:${s.shooter}:${endKey(s.path[0])}`),
            ),
          ) < probability);
      if (visible) this.tracers.add(s);
      if (s.hit === "none") {
        const key = flightKey(s, s.path[s.path.length - 1]);
        const choices = flying.get(key);
        if (choices) choices.push(visible);
        else flying.set(key, [visible]);
      }
    }
    this.flying = flying;
    return out;
  }

  /** A hull's shot from mount `m`: from its own muzzle, along its aim. Its
   *  pivot turns with its carrier (the mount it is on, or the hull). */
  private hullLaunch(u: EffectShooter, m: number): Launch {
    const mount = u.mounts[m];
    const at = _launch_offset;
    if (mount.muzzle) {
      const carried = mount.muzzle.on === null ? u.yaw : u.mounts[mount.muzzle.on].bearing;
      muzzleOffset(at, mount.muzzle, carried, mount.bearing);
    } else vec3.set(at, 0, 0, 0);
    const c = Math.cos(mount.bearing);
    const s = Math.sin(mount.bearing);
    const ce = Math.cos(mount.elevation);
    return {
      shooter: u.key,
      mount: m,
      soldier: null,
      x: u.position[0] + at[0],
      y: u.position[1] + at[1],
      z: u.position[2] + at[2],
      dx: ce * c,
      dy: ce * s,
      dz: Math.sin(mount.elevation),
      kind: mount.kind,
      hull: true,
    };
  }
}
