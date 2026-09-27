// Which shots a side's publications show, the one derivation both the
// muzzle flashes (`EffectFrame`) and the gunfire sounds (`battle-audio`)
// read. A mount's shot counter rising is a shot: a hull fires from its
// muzzle along the mount's bearing and elevation; a squad's rise goes to the
// soldiers who start new rounds this tick, one round each, matched by kind.
// A round still flying (its stretch starts where last tick's ended) is not a
// new launch. A unit first seen, or seen again, shows no shot until its
// counter rises once more.
import type { EffectPublication, EffectSegment } from "./effectFrame";

type P3 = readonly [number, number, number] | readonly number[];

/** One shot the publication shows, at the start of its tick. */
export interface Launch {
  /** The shooter's key (`EffectShooter.key`). */
  shooter: number;
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

export class LaunchTracker {
  /** Shot counters by shooter key, as last published. */
  private counters = new Map<number, number[]>();
  /** Where the last publication's still-flying stretches ended. */
  private flying = new Set<string>();

  /** `muzzle`: a hull's muzzle in its turret's frame (forward, left, up). */
  constructor(private readonly muzzle: P3) {}

  reset() {
    this.counters.clear();
    this.flying.clear();
  }

  /** The launches `pub` shows. Call once per publication, in order; `gap`
   *  says ticks were skipped since the last one. */
  note(pub: EffectPublication, gap: boolean): Launch[] {
    const out: Launch[] = [];
    const starts = new Map<number, EffectSegment[]>();
    for (const s of pub.segments) {
      if (s.shooter === null || s.path.length < 2) continue;
      if (!gap && this.flying.has(endKey(s.path[0]))) continue; // a round still flying
      const list = starts.get(s.shooter);
      if (list) list.push(s);
      else starts.set(s.shooter, [s]);
    }
    const seen = new Set<number>();
    for (const u of pub.shooters) {
      seen.add(u.key);
      const before = this.counters.get(u.key);
      this.counters.set(
        u.key,
        u.mounts.map((m) => m.shots),
      );
      if (!before) continue; // first seen: no shot to show
      for (let m = 0; m < u.mounts.length; m++) {
        const mount = u.mounts[m];
        let rose = mount.shots - (before[m] ?? mount.shots);
        if (rose <= 0) continue;
        if (u.half) {
          const [f, l, h] = u.muzzle ?? this.muzzle;
          const c = Math.cos(mount.bearing);
          const s = Math.sin(mount.bearing);
          const ce = Math.cos(mount.elevation);
          out.push({
            shooter: u.key,
            x: u.position[0] + f * c - l * s,
            y: u.position[1] + f * s + l * c,
            z: u.position[2] + h,
            dx: ce * c,
            dy: ce * s,
            dz: Math.sin(mount.elevation),
            kind: mount.kind,
            hull: true,
          });
          continue;
        }
        for (const id of u.members) {
          for (const s of starts.get(id) ?? []) {
            if (rose <= 0 || s.kind !== mount.kind) continue;
            rose--;
            const [a, b] = [s.path[0], s.path[1]];
            const len = Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2]) || 1;
            out.push({
              shooter: u.key,
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
    // Units no longer seen start over when they are seen again.
    for (const key of this.counters.keys()) if (!seen.has(key)) this.counters.delete(key);
    this.flying = new Set();
    for (const s of pub.segments)
      if (s.path.length >= 2 && s.hit === "none")
        this.flying.add(endKey(s.path[s.path.length - 1]));
    return out;
  }
}
