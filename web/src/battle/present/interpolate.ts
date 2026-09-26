/** Presentation between completed ticks: own units drawn a fraction of a tick
 * behind the latest publication, blended from the previous one. Only units
 * present in both frames blend; nothing is extrapolated. */
import { deltaAngle, lerp, vec3 } from "math";
import type { ObservationView, OwnUnitView, Point3 } from "../sim/observation";

export interface Pose {
  id: number;
  position: Point3;
  yaw: number;
  members: Point3[];
  /** Deployment progress in [0, 1], blended like the body; null if none. */
  deployment: number | null;
}

/** Turns the short way round from `a` toward `b`. */
function lerpAngle(a: number, b: number, t: number) {
  return a + deltaAngle(a, b) * t;
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
      const deployment = u.deployment?.progress ?? null;
      if (!p) return { id: u.id, position: u.position, yaw: u.yaw, members: u.members, deployment };
      return {
        id: u.id,
        position: vec3.lerp(vec3.create(), p.position, u.position, t),
        yaw: lerpAngle(p.yaw, u.yaw, t),
        members:
          p.members.length === u.members.length
            ? u.members.map((m, k) => vec3.lerp(vec3.create(), p.members[k], m, t))
            : u.members,
        deployment:
          deployment !== null && p.deployment
            ? lerp(p.deployment.progress, deployment, t)
            : deployment,
      };
    });
  }
}
