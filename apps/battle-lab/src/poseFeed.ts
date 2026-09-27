// The observation, as the pose driver reads it. A side's decoded observation
// and its interpolated poses become a `FeedFrame`: every own unit and every
// identified enemy with its soldiers by member id, its weapon poses, and the
// fallen the side knows of. Each soldier's position is his own, blended by
// his id between ticks, so a soldier's velocity is his own and never a
// formation slot's (README firewalls).
//
// The same rules also give the driver its facts: mount roles, the vehicles'
// track gauge, the suppression at which soldiers go prone, and each kind's
// clips from the installed appearances.
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  PoseDriver,
  type FeedFallen,
  type FeedFrame,
  type FeedMount,
  type FeedUnit,
  type MountRole,
  type UnitKindName,
} from "@packages/battle-renderer/src/models/poseDriver";
import type { Pose } from "@web/battle/present/interpolate";
import type { ObservationView, WeaponPoseView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";

/** The rule blocks the pose driver reads (the scenario's or the fixture's). */
export interface PoseRules {
  mounts: Record<string, { name: string; turret?: boolean }[]>;
  physics: {
    tank_half_extents_m: number[];
    supply_half_extents_m: number[];
    jeep_half_extents_m: number[];
  };
  suppression: { collapse_level: number };
}

const KINDS: readonly UnitKindName[] = ["rifle", "recon", "at", "tank", "supply", "jeep"];
const isKind = (kind: string): kind is UnitKindName => (KINDS as readonly string[]).includes(kind);

/** Mount roles per unit kind from the rules' mount lists: the first turret
 *  mount is the gun, an HMG turret mount the HMG, anything else in hand. */
export function mountRoles(
  mounts: PoseRules["mounts"],
): Partial<Record<UnitKindName, MountRole[]>> {
  const out: Partial<Record<UnitKindName, MountRole[]>> = {};
  for (const [kind, list] of Object.entries(mounts)) {
    if (!isKind(kind)) continue;
    let gun = false;
    out[kind] = list.map((m) => {
      if (!m.turret) return "hand";
      if (/hmg/i.test(m.name)) return "hmg";
      if (!gun) {
        gun = true;
        return "gun";
      }
      return "hand";
    });
  }
  return out;
}

/** Half the gauge of each vehicle kind's running gear, from its hit box. */
export function halfTrack(physics: PoseRules["physics"]): Partial<Record<UnitKindName, number>> {
  return {
    tank: physics.tank_half_extents_m[1] * 0.8,
    supply: physics.supply_half_extents_m[1] * 0.75,
    jeep: physics.jeep_half_extents_m[1] * 0.75,
  };
}

/** A pose driver for `rules`, reading each kind's clips from its appearance. */
export function createPoseDriver(rules: PoseRules, installed: InstalledAppearances): PoseDriver {
  const catalog = new AppearanceCatalog(installed);
  return new PoseDriver({
    mounts: mountRoles(rules.mounts),
    halfTrack: halfTrack(rules.physics),
    pinned: rules.suppression.collapse_level,
    clip: (kind, name) => {
      const resolved = catalog.resolve(kind, "blue");
      const bundle = resolved && installed.appearances.get(resolved.appearance)?.bundle;
      if (bundle?.kind !== "skinned") return null;
      const clip = installed.skeletons.get(bundle.skeleton)?.clips.find((c) => c.name === name);
      return clip
        ? { duration: clip.duration, loop: clip.loop, stride_m: clip.stride_m ?? null }
        : null;
    },
  });
}

/** Mount records in the rules' mount order, from a unit's weapon poses. */
function mountsOf(poses: readonly WeaponPoseView[], into: FeedMount[]): FeedMount[] {
  into.length = 0;
  for (const p of poses)
    into[p.mount] = { bearing: p.bearing, elevation: p.elevation, shots: p.shots };
  for (let i = 0; i < into.length; i++) into[i] ??= { bearing: 0, elevation: 0, shots: 0 };
  return into;
}

/** Turns one side's observations into feed frames. Per observation it keeps
 *  the fallen list (the same array until the next publication, so the driver
 *  reconciles corpses only then) and the soldiers named by visible rounds. */
export class ObservationFeed {
  private observation: ObservationView | null = null;
  private fallen: FeedFallen[] = [];
  private shooters = new Set<number>();
  private readonly mounts = new Map<number, FeedMount[]>();

  constructor(private readonly side: SideName) {}

  /** The frame at presentation time `time` (simulation seconds), from
   *  `observation` and the own and identified poses interpolated for it. */
  frame(
    observation: ObservationView,
    own: readonly Pose[],
    identified: readonly Pose[],
    time: number,
  ): FeedFrame {
    const enemy: SideName = this.side === "blue" ? "red" : "blue";
    if (observation !== this.observation) {
      this.observation = observation;
      this.fallen = observation.corpses.flatMap((c) =>
        isKind(c.kind)
          ? [
              {
                soldier: c.soldier,
                position: c.position,
                yaw: c.yaw,
                kind: c.kind,
                side: c.own ? this.side : enemy,
              },
            ]
          : [],
      );
      this.shooters = new Set();
      for (const p of observation.projectiles)
        if (p.shooterMember !== null) this.shooters.add(p.shooterMember);
    }
    const units: FeedUnit[] = [];
    const byId = new Map(observation.own.map((u) => [u.id, u]));
    for (const pose of own) {
      const u = byId.get(pose.id);
      if (u && isKind(u.kind))
        units.push(this.unit(pose, u.kind, this.side, u.weaponPoses, u.suppression));
    }
    const enemies = new Map(observation.identified.map((e) => [e.id, e]));
    for (const pose of identified) {
      const e = enemies.get(pose.id);
      // The side cannot know an enemy's suppression.
      if (e && isKind(e.kind)) units.push(this.unit(pose, e.kind, enemy, e.weaponPoses, 0));
    }
    return { time, units, fallen: this.fallen };
  }

  private unit(
    pose: Pose,
    kind: UnitKindName,
    side: SideName,
    weaponPoses: readonly WeaponPoseView[],
    suppression: number,
  ): FeedUnit {
    let mounts = this.mounts.get(pose.id);
    if (!mounts) this.mounts.set(pose.id, (mounts = []));
    return {
      id: pose.id,
      kind,
      side,
      position: pose.position,
      yaw: pose.yaw,
      soldiers: pose.members.map((position, k) => ({
        id: pose.memberIds[k],
        position,
        shooting: this.shooters.has(pose.memberIds[k]),
      })),
      mounts: mountsOf(weaponPoses, mounts),
      deployment: pose.deployment,
      suppression,
    };
  }
}
