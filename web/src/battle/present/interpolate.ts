/** Presentation between completed ticks: units drawn a fraction of a tick
 * behind the latest publication, blended from the previous one. Only units
 * present in both frames blend, and a soldier blends only with himself (by
 * member id); nothing is extrapolated. Own units and identified enemies blend
 * alike, so an enemy squad walks as smoothly as one's own. */
import { deltaAngle, lerp, vec3 } from "math";
import type { ObservationView, Point3 } from "../sim/observation";

export interface Pose {
  id: number;
  position: Point3;
  yaw: number;
  members: Point3[];
  /** Each member's soldier id, in `members` order. */
  memberIds: number[];
  /** Deployment progress in [0, 1], blended like the body; null if none. */
  deployment: number | null;
}

/** What both own units and identified enemies publish about their bodies. */
interface Body {
  id: number;
  position: Point3;
  yaw: number;
  members: Point3[];
  memberIds: number[];
}

/** Turns the short way round from `a` toward `b`. */
function lerpAngle(a: number, b: number, t: number) {
  return a + deltaAngle(a, b) * t;
}

/** `u` blended from `p` (the previous tick's same unit, if any) by `t`. */
function blendBody(p: Body | undefined, u: Body, t: number, deployment: number | null): Pose {
  if (!p)
    return {
      id: u.id,
      position: u.position,
      yaw: u.yaw,
      members: u.members,
      memberIds: u.memberIds,
      deployment,
    };
  const before = new Map<number, Point3>();
  p.memberIds.forEach((id, k) => before.set(id, p.members[k]));
  return {
    id: u.id,
    position: vec3.lerp(vec3.create(), p.position, u.position, t),
    yaw: lerpAngle(p.yaw, u.yaw, t),
    members: u.members.map((m, k) => {
      const was = before.get(u.memberIds[k]);
      return was ? vec3.lerp(vec3.create(), was, m, t) : m;
    }),
    memberIds: u.memberIds,
    deployment,
  };
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

  /** How far from the previous tick toward the latest `now` is, in [0, 1]. */
  private fraction(now: number): number {
    return Math.min(1, Math.max(0, (now - this.latestAt) / this.tickMs));
  }

  /** The presentation clock, in simulation seconds: the tick being shown at
   *  `now`, fractional (the previous tick blended toward the latest, as the
   *  poses are). It stands still while no tick arrives, so a paused battle's
   *  poses and wind hold. The one presentation clock: it drives the pose
   *  driver and the frame's `setClock` alike. Null before the first publication. */
  time(now: number): number | null {
    const latest = this.latest;
    if (!latest) return null;
    const from = this.previous?.tick ?? latest.tick;
    return ((from + (latest.tick - from) * this.fraction(now)) * this.tickMs) / 1000;
  }

  /** Own units' poses at `now`: the previous tick blended toward the latest one. */
  sample(now: number): Pose[] {
    const latest = this.latest;
    if (!latest) return [];
    const t = this.fraction(now);
    const before = new Map((this.previous?.own ?? []).map((u) => [u.id, u]));
    return latest.own.map((u) => {
      const p = before.get(u.id);
      const deployment = u.deployment?.progress ?? null;
      return blendBody(
        p,
        u,
        t,
        deployment !== null && p?.deployment
          ? lerp(p.deployment.progress, deployment, t)
          : deployment,
      );
    });
  }

  /** Identified enemies' poses at `now`, blended the same way. */
  sampleIdentified(now: number): Pose[] {
    const latest = this.latest;
    if (!latest) return [];
    const t = this.fraction(now);
    const before = new Map((this.previous?.identified ?? []).map((u) => [u.id, u]));
    return latest.identified.map((u) => blendBody(before.get(u.id), u, t, null));
  }
}
