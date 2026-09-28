// The observation, as the pose driver reads it. A side's decoded observation
// and its interpolated poses become a `FeedFrame`: every own unit and every
// identified enemy with its soldiers by member id, its weapon poses, and the
// fallen the side knows of. Each soldier's position is his own, blended by
// his id between ticks, so a soldier's velocity is his own and never a
// formation slot's (README firewalls).
//
// The unit catalog gives the driver its facts (which units are vehicles,
// their mount roles and track gauge), the rules the suppression at which
// soldiers go prone, the installed appearances each type's clips, and
// `presentation.pose` its feel.
import village from "@fixtures/village.json";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  PoseDriver,
  validatePoseFeel,
  type FeedFallen,
  type FeedFrame,
  type FeedMount,
  type FeedUnit,
  type PoseFeel,
} from "@packages/battle-renderer/src/models/poseDriver";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { LaunchTracker } from "@packages/battle-renderer/src/effects/launches";
import type { Pose } from "@web/battle/present/interpolate";
import type { ObservationView, WeaponPoseView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";
import { effectPublication } from "./effectFeed";

/** The rule blocks the pose driver reads (the scenario's or the fixture's). */
export interface PoseRules {
  suppression: { collapse_level: number };
}

/** `presentation.pose`: the pose driver's feel. */
export const villagePose: PoseFeel = validatePoseFeel(
  village.presentation.pose as unknown as PoseFeel,
);

/** A pose driver for `rules` and the unit catalog, reading each type's clips
 *  from its appearance. */
export function createPoseDriver(
  rules: PoseRules,
  units: UnitCatalog,
  installed: InstalledAppearances,
): PoseDriver {
  const catalog = new AppearanceCatalog(installed, units);
  return new PoseDriver({
    units,
    pinned: rules.suppression.collapse_level,
    feel: villagePose,
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

/** What a unit's publication says of its soldiers and mounts: an own unit's
 *  or a seen enemy's. */
type Published = Pick<
  ObservationView["own"][number],
  "memberIds" | "memberSlots" | "memberLeans" | "weaponPoses"
>;

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
 *  reconciles corpses only then) and the soldiers who fired: the launches
 *  (`launches.ts`) the publication shows, the one derivation the muzzle
 *  flashes and the gunfire sounds read too, so the soldier who kneels to
 *  fire is the one whose flash and sound fire. */
export class ObservationFeed {
  private observation: ObservationView | null = null;
  private fallen: FeedFallen[] = [];
  private shooters = new Set<number>();
  private readonly launches = new LaunchTracker();
  private lastTick = -1;
  private readonly mounts = new Map<number, FeedMount[]>();

  constructor(
    private readonly side: SideName,
    private readonly units: UnitCatalog,
  ) {}

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
      this.fallen = observation.corpses.map((c) => ({
        soldier: c.soldier,
        position: c.position,
        yaw: c.yaw,
        kind: c.kind,
        slot: c.slot,
        side: c.own ? this.side : enemy,
      }));
      this.shooters = new Set();
      if (observation.tick !== this.lastTick) {
        if (observation.tick < this.lastTick) this.launches.reset();
        const gap = this.lastTick >= 0 && observation.tick !== this.lastTick + 1;
        this.lastTick = observation.tick;
        const pub = effectPublication(observation, this.side, this.units);
        for (const l of this.launches.note(pub, gap))
          if (l.soldier !== null) this.shooters.add(l.soldier);
      }
    }
    const units: FeedUnit[] = [];
    const byId = new Map(observation.own.map((u) => [u.id, u]));
    for (const pose of own) {
      const u = byId.get(pose.id);
      if (u) units.push(this.unit(pose, u.kind, this.side, u, u.suppression));
    }
    const enemies = new Map(observation.identified.map((e) => [e.id, e]));
    for (const pose of identified) {
      const e = enemies.get(pose.id);
      // The side cannot know an enemy's suppression.
      if (e) units.push(this.unit(pose, e.kind, enemy, e, 0));
    }
    return { time, units, fallen: this.fallen };
  }

  private unit(
    pose: Pose,
    kind: string,
    side: SideName,
    published: Published,
    suppression: number,
  ): FeedUnit {
    let mounts = this.mounts.get(pose.id);
    if (!mounts) this.mounts.set(pose.id, (mounts = []));
    // Leans are discrete (out or in): read from the observation by soldier
    // id, never interpolated; the driver eases the slide.
    const lean = (id: number) => {
      const k = published.memberIds.indexOf(id);
      return k < 0 ? null : (published.memberLeans[k]?.at ?? null);
    };
    const slot = (id: number) => published.memberSlots[published.memberIds.indexOf(id)] ?? 0;
    return {
      id: pose.id,
      kind,
      side,
      position: pose.position,
      yaw: pose.yaw,
      soldiers: pose.members.map((position, k) => ({
        id: pose.memberIds[k],
        slot: slot(pose.memberIds[k]),
        position,
        shooting: this.shooters.has(pose.memberIds[k]),
        lean: lean(pose.memberIds[k]),
      })),
      mounts: mountsOf(published.weaponPoses, mounts),
      deployment: pose.deployment,
      suppression,
    };
  }
}
