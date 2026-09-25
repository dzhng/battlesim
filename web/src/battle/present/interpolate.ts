/** Presentation between completed ticks: own units drawn a fraction of a tick
 * behind the latest publication, blended from the previous one. Only units
 * present in both frames blend; nothing is extrapolated. */
import type { ObservationView, OwnUnitView, Point3 } from "../sim/observation";

export interface Pose {
  id: number;
  position: Point3;
  yaw: number;
  members: Point3[];
}

function lerp(a: number, b: number, t: number) {
  return a + (b - a) * t;
}

function lerpAngle(a: number, b: number, t: number) {
  const d = Math.atan2(Math.sin(b - a), Math.cos(b - a));
  return a + d * t;
}

function lerp3(a: Point3, b: Point3, t: number): Point3 {
  return [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)];
}

export class TickInterpolator {
  private previous: ObservationView | null = null;
  private latest: ObservationView | null = null;
  private latestAt = 0;

  constructor(private readonly tickMs: number) {}

  push(observation: ObservationView, receivedAt: number) {
    this.previous = this.latest;
    this.latest = observation;
    this.latestAt = receivedAt;
  }

  /** Poses at `now`: the previous tick blended toward the latest one. */
  sample(now: number): Pose[] {
    const latest = this.latest;
    if (!latest) return [];
    const t = Math.min(1, Math.max(0, (now - this.latestAt) / this.tickMs));
    const before = new Map<number, OwnUnitView>((this.previous?.own ?? []).map((u) => [u.id, u]));
    return latest.own.map((u) => {
      const p = before.get(u.id);
      if (!p) return { id: u.id, position: u.position, yaw: u.yaw, members: u.members };
      return {
        id: u.id,
        position: lerp3(p.position, u.position, t),
        yaw: lerpAngle(p.yaw, u.yaw, t),
        members:
          p.members.length === u.members.length
            ? u.members.map((m, k) => lerp3(p.members[k], m, t))
            : u.members,
      };
    });
  }
}
