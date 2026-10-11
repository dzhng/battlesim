// The observation, as the pose driver reads it. A side's decoded observation
// and its interpolated poses become a `FeedFrame`: every own unit and every
// identified enemy with its soldiers by member id, its weapon poses, every
// downed airframe still falling, and the fallen the side knows of. Each soldier's position is his own, blended by
// his id between ticks, so a soldier's velocity is his own and never a
// formation slot's (README firewalls).
//
// The unit catalog gives the driver its facts (which units are vehicles,
// their mount roles and track gauge), the installed appearances each type's
// clips, and `presentation.pose` its feel. Whether a squad is pinned (its men
// lie flat or crouch) is the published suppression tier.
import game from "@fixtures/game.json";
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
import { sideKey } from "@packages/battle-renderer/src/sideKey";
import type { FrameSample, Pose } from "@web/battle/present/interpolate";
import type { ObservationView, WeaponPoseView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";
import { effectPublication } from "./effectFeed";

/** The rule blocks the pose driver reads (the scenario's or the fixture's). */
export interface PoseRules {
  cover: { lean_hold_s: number };
}

/** `presentation.pose`: the pose driver's feel. */
export const gamePose: PoseFeel = validatePoseFeel(game.presentation.pose as unknown as PoseFeel);

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
    mounts: (kind) => catalog.mountRoles(kind),
    feel: gamePose,
    leanHold: rules.cover.lean_hold_s,
    clip: (kind, name, soldier) => {
      const resolved = catalog.resolve(
        kind,
        "blue",
        soldier?.soldier ?? 0,
        soldier?.slot ?? 0,
        soldier?.operatorMount ?? null,
        soldier?.activeMount ?? null,
      );
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
  "memberIds" | "memberSlots" | "memberActiveMounts" | "memberLeans" | "weaponPoses"
>;

/** Mount records in the rules' mount order, from a unit's weapon poses. */
function mountsOf(poses: readonly WeaponPoseView[], into: FeedMount[]): FeedMount[] {
  into.length = 0;
  for (const p of poses)
    into[p.mount] = {
      operator: p.operator,
      bearing: p.bearing,
      elevation: p.elevation,
      shots: p.shots,
    };
  for (let i = 0; i < into.length; i++)
    into[i] ??= { operator: null, bearing: 0, elevation: 0, shots: 0 };
  return into;
}

/** Turns one side's observations into feed frames. Keeps the fallen list
 *  until the corpse view changes, so the driver reconciles only those changes,
 *  and counts each soldier's shots: the
 *  launches (`launches.ts`) each publication shows, the one derivation the
 *  muzzle flashes and the gunfire sounds read too, so the soldier who kneels
 *  to fire is the one whose flash and sound fire. */
export class ObservationFeed {
  private observation: ObservationView | null = null;
  private fallen: FeedFallen[] = [];
  private corpses: ObservationView["corpses"] | null = null;
  /** Each soldier's launches so far, by soldier id. */
  private readonly shots = new Map<number, number[]>();
  private readonly launches = new LaunchTracker();
  private lastTick = -1;
  /** Each unit's mount records, by `sideKey`: an identified enemy's handle
   *  can equal an own unit's id. */
  private readonly mounts = new Map<number, FeedMount[]>();

  constructor(
    private readonly side: SideName,
    private readonly units: UnitCatalog,
  ) {}

  /** The frame for one drawn sample: its poses and the publication they
   *  blend toward, never an older one, so a soldier who has left the poses
   *  is already among the fallen and plays his death. */
  frame({ observation, own, identified, crashes, time }: FrameSample): FeedFrame {
    const enemy: SideName = this.side === "blue" ? "red" : "blue";
    if (observation !== this.observation) {
      this.observation = observation;
      if (observation.corpses !== this.corpses) {
        this.corpses = observation.corpses;
        this.fallen = observation.corpses.map((c) => ({
          soldier: c.soldier,
          position: c.position,
          yaw: c.yaw,
          kind: c.kind,
          slot: c.slot,
          side: c.own ? this.side : enemy,
        }));
      }
      if (observation.tick !== this.lastTick) {
        if (observation.tick < this.lastTick) {
          this.launches.reset();
          this.shots.clear();
        }
        const gap = this.lastTick >= 0 && observation.tick !== this.lastTick + 1;
        this.lastTick = observation.tick;
        const pub = effectPublication(observation, this.side, this.units);
        for (const l of this.launches.note(pub, gap)) {
          if (l.soldier === null) continue;
          let counts = this.shots.get(l.soldier);
          if (!counts) this.shots.set(l.soldier, (counts = []));
          counts[l.mount] = (counts[l.mount] ?? 0) + 1;
        }
      }
    }
    const units: FeedUnit[] = [];
    const byId = new Map(observation.own.map((u) => [u.id, u]));
    for (const pose of own) {
      const u = byId.get(pose.id);
      if (u) units.push(this.unit(pose, u.kind, this.side, u, u.suppression === "pinned"));
    }
    const enemies = new Map(observation.identified.map((e) => [e.id, e]));
    for (const pose of identified) {
      const e = enemies.get(pose.id);
      // The side cannot know an enemy's suppression.
      if (e) units.push(this.unit(pose, e.kind, enemy, e, false));
    }
    // A downed airframe falls on as the vehicle it was (its id is the one the
    // side knew it by), crewless, its turret as last seen, tipping into the fall.
    for (const c of crashes) {
      const side = c.own ? this.side : enemy;
      units.push({
        id: c.id,
        kind: c.kind,
        side,
        position: c.position,
        yaw: c.yaw,
        soldiers: [],
        mounts: this.mounts.get(sideKey(c.id, side, "blue")) ?? [],
        deployment: null,
        pinned: false,
        attitude: { pitch: c.pitch, roll: c.roll },
      });
    }
    return { time, units, fallen: this.fallen };
  }

  private unit(
    pose: Pose,
    kind: string,
    side: SideName,
    published: Published,
    pinned: boolean,
  ): FeedUnit {
    const key = sideKey(pose.id, side, "blue");
    let mounts = this.mounts.get(key);
    if (!mounts) this.mounts.set(key, (mounts = []));
    return {
      id: pose.id,
      kind,
      side,
      position: pose.position,
      yaw: pose.yaw,
      soldiers: pose.members.map((position, k) => {
        const id = pose.memberIds[k];
        const publishedIndex = published.memberIds.indexOf(id);
        const activeMount = published.memberActiveMounts[publishedIndex] ?? null;
        return {
          id,
          slot: published.memberSlots[publishedIndex] ?? 0,
          activeMount,
          position,
          shots: activeMount === null ? 0 : (this.shots.get(id)?.[activeMount] ?? 0),
          // Leans are discrete: the driver eases the slide from this visible fact.
          lean: published.memberLeans[publishedIndex]?.at ?? null,
        };
      }),
      mounts: mountsOf(published.weaponPoses, mounts),
      deployment: pose.deployment,
      pinned,
    };
  }
}
