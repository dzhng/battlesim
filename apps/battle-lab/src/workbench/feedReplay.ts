// A feed replay: synthetic observation frames for one unit of a type, scripted
// in time, published at the simulation's tick rate and interpolated between
// ticks the way the battle interpolates presentation. It drives the real pose
// driver, so the workbench shows what a model does when the simulation walks,
// runs, fires, is pinned, dies, drives, turns, slews, and deploys. The script
// follows the type's components: a squad walks and fights, a deploying hull
// sets up, a hull with a turret gun slews and fires it, one with only a
// machine gun rakes with it.
//
// Soldiers move on their own paths (each his own offset, speed and delay),
// never as a formation, so the replay exercises per-soldier posing.

import { clamp, lerp } from "math";
import game from "@fixtures/game.json";
import type {
  FeedFrame,
  FeedMount,
  FeedUnit,
} from "@packages/battle-renderer/src/models/poseDriver";
import type { MountRole, UnitCatalog } from "@packages/scene-assets/src/units";

export const TICK_HZ = game.tick_hz;

/** The unit the replay drives: its type, and how the model on the bench
 *  draws each of its mounts (`mountRoles`). */
export interface ReplayUnit {
  kind: string;
  mounts: readonly MountRole[];
  /** The catalog `kind` is read from. */
  units: UnitCatalog;
}

/** One scripted beat: from `at` seconds, what the unit is doing. */
interface Beat {
  at: number;
  name: string;
}

export const INFANTRY_BEATS: Beat[] = [
  { at: 0, name: "idle" },
  { at: 2, name: "walk" },
  { at: 6, name: "run" },
  { at: 9, name: "fire" },
  { at: 12.5, name: "pinned" },
  { at: 16, name: "fall" },
  { at: 20, name: "end" },
];
/** A hull with a turret gun (and perhaps a roof HMG on it). */
export const GUN_BEATS: Beat[] = [
  { at: 0, name: "idle" },
  { at: 1, name: "drive" },
  { at: 5, name: "turn" },
  { at: 8, name: "slew" },
  { at: 11, name: "fire" },
  { at: 13, name: "hmg" },
  { at: 18, name: "end" },
];
/** A hull that deploys in place. */
export const DEPLOY_BEATS: Beat[] = [
  { at: 0, name: "idle" },
  { at: 1, name: "drive" },
  { at: 5, name: "deploy" },
  { at: 15, name: "deployed" },
  { at: 17, name: "pack" },
  { at: 24, name: "end" },
];

/** A hull whose one weapon is a machine gun on its own ring. */
export const HMG_BEATS: Beat[] = [
  { at: 0, name: "idle" },
  { at: 1, name: "drive" },
  { at: 4, name: "turn" },
  { at: 6, name: "slew" },
  { at: 8, name: "hmg" },
  { at: 14, name: "end" },
];

type Script = "infantry" | "deploy" | "gun" | "hmg";

/** Which script a unit type's components call for. */
function scriptFor({ kind, mounts, units }: ReplayUnit): Script {
  if (!units.hull(kind)) return "infantry";
  if (units.type(kind).capabilities.deploy) return "deploy";
  return mounts.includes("gun") ? "gun" : "hmg";
}

export function beatsFor(replay: ReplayUnit): Beat[] {
  return { infantry: INFANTRY_BEATS, deploy: DEPLOY_BEATS, gun: GUN_BEATS, hmg: HMG_BEATS }[
    scriptFor(replay)
  ];
}

export function beatAt(replay: ReplayUnit, t: number): string {
  const beats = beatsFor(replay);
  let name = beats[0].name;
  for (const b of beats) if (t >= b.at) name = b.name;
  return name;
}

export function replayLength(replay: ReplayUnit): number {
  return beatsFor(replay).at(-1)!.at;
}

const ramp = (t: number, t0: number, t1: number) => clamp((t - t0) / (t1 - t0), 0, 1);

/** Distance a soldier has covered by `t`: walking 1.4 m/s, then running 4 m/s. */
function infantryPath(t: number, delay: number): number {
  const s = t - delay;
  const walk = Math.min(Math.max(0, s - 2), 4) * 1.4;
  const run = Math.min(Math.max(0, s - 6), 3) * 4;
  return walk + run;
}

/** Each of the type's mounts' poses, from the script's gun and HMG: a hand
 *  weapon holds still. */
function mountsOf(
  roles: readonly MountRole[],
  gun: FeedMount | null,
  hmg: FeedMount | null,
): FeedMount[] {
  const still: FeedMount = { bearing: 0, elevation: 0, shots: 0 };
  return roles.map((role) =>
    role === "gun" ? (gun ?? still) : role === "hmg" ? (hmg ?? still) : still,
  );
}

/** The unit at an exact tick time. */
function unitAt(replay: ReplayUnit, t: number, soldiers: number): FeedUnit {
  const { kind, units } = replay;
  const script = scriptFor(replay);
  if (script === "gun") {
    const drive = Math.min(Math.max(0, t - 1), 4) * 5;
    const yaw = lerp(0, Math.PI / 2, ramp(t, 5, 8));
    const bearing = yaw + lerp(0, -1.2, ramp(t, 8, 11));
    const cannon: FeedMount = {
      bearing,
      elevation: lerp(0, 0.12, ramp(t, 9, 11)),
      shots: t >= 11.2 ? 1 + Math.floor((t - 11.2) / 4) : 0,
    };
    const hmg: FeedMount = {
      bearing: bearing + (t >= 13 ? Math.sin((t - 13) * 1.3) * 0.9 : 0),
      elevation: t >= 13 ? 0.1 + 0.2 * Math.sin((t - 13) * 0.7) : 0,
      shots: t >= 13 ? Math.floor((t - 13) * 6) : 0,
    };
    return {
      id: 1,
      kind,
      side: "blue",
      position: [drive, 0, 0],
      yaw,
      soldiers: [],
      mounts: mountsOf(replay.mounts, cannon, hmg),
      deployment: null,
      pinned: false,
    };
  }
  if (script === "hmg") {
    // Drives, turns, then slews its HMG right round and fires.
    const drive = Math.min(Math.max(0, t - 1), 3) * 8;
    const yaw = lerp(0, Math.PI / 2, ramp(t, 4, 6));
    const hmg: FeedMount = {
      bearing: yaw + lerp(0, Math.PI, ramp(t, 6, 8)) + (t >= 8 ? Math.sin((t - 8) * 1.1) * 0.6 : 0),
      elevation: t >= 8 ? 0.08 + 0.15 * Math.sin((t - 8) * 0.7) : 0,
      shots: t >= 8 ? Math.floor((t - 8) * 6) : 0,
    };
    return {
      id: 1,
      kind,
      side: "blue",
      position: [drive, 0, 0],
      yaw,
      soldiers: [],
      mounts: mountsOf(replay.mounts, null, hmg),
      deployment: null,
      pinned: false,
    };
  }
  if (script === "deploy") {
    const drive = Math.min(Math.max(0, t - 1), 4) * 4;
    const deployment = t < 17 ? ramp(t, 5, 15) : 1 - ramp(t, 17, 24);
    return {
      id: 1,
      kind,
      side: "blue",
      position: [drive, 0, 0],
      yaw: 0,
      soldiers: [],
      mounts: mountsOf(replay.mounts, null, null),
      deployment,
      pinned: false,
    };
  }
  // Infantry: each soldier on his own lane, speed profile and delay, in
  // his slot of the squad type.
  const slots = units.slots(kind).length;
  const members = Array.from({ length: soldiers }, (_, i) => {
    const delay = i * 0.35;
    const lane = (i - (soldiers - 1) / 2) * 1.6;
    const along = infantryPath(t, delay) * (1 - i * 0.04);
    return {
      id: 100 + i,
      slot: i % slots,
      activeMount: 0,
      position: [along, lane, 0] as [number, number, number],
    };
  });
  const firing = t >= 9.2 && t < 12.5;
  return {
    id: 1,
    kind,
    side: "blue",
    position: members[0].position,
    yaw: 0,
    soldiers: members,
    // Synthetic replay soldiers use their default hand weapon.
    mounts: units.type(kind).mounts.map((_, mount) => ({
      bearing: 0.2,
      elevation: 0.02,
      shots:
        mount === 0
          ? firing
            ? Math.floor((t - 9.2) / 0.6) * soldiers
            : t >= 12.5
              ? 6 * soldiers
              : 0
          : 0,
    })),
    deployment: null,
    // Pinned for a while, then recovering.
    pinned: t >= 12.5 && t < 16,
  };
}

/** The published frame at tick `k`, with the soldiers who have fallen. */
export function feedTick(replay: ReplayUnit, k: number, soldiers = 4): FeedFrame {
  const { kind } = replay;
  const t = k / TICK_HZ;
  const unit = unitAt(replay, t, soldiers);
  const fallen =
    scriptFor(replay) === "infantry" && t >= 16
      ? unit.soldiers.slice(0, 1).map((s) => ({
          soldier: s.id,
          position: s.position,
          yaw: 0.3,
          kind,
          slot: s.slot,
          side: "blue" as const,
        }))
      : [];
  if (fallen.length) unit.soldiers = unit.soldiers.filter((s) => s.id !== fallen[0].soldier);
  return { time: t, units: [unit], fallen };
}

/** The frame the presentation shows at `t`: positions and bearings eased
 *  between the two ticks around it, everything else from the earlier tick. */
export function feedAt(replay: ReplayUnit, t: number, soldiers = 4): FeedFrame {
  const k = Math.floor(t * TICK_HZ);
  const a = feedTick(replay, k, soldiers);
  const b = feedTick(replay, k + 1, soldiers);
  const w = t * TICK_HZ - k;
  const mix = (p: number[], q: number[]) =>
    p.map((v, i) => lerp(v, q[i], w)) as [number, number, number];
  const units = a.units.map((u, i) => {
    const v = b.units[i];
    return {
      ...u,
      position: mix(u.position, v.position),
      yaw: lerp(u.yaw, v.yaw, w),
      soldiers: u.soldiers.map((s) => {
        const next = v.soldiers.find((n) => n.id === s.id);
        return next ? { ...s, position: mix(s.position, next.position) } : s;
      }),
      mounts: u.mounts.map((m, j) => ({
        ...m,
        bearing: lerp(m.bearing, v.mounts[j].bearing, w),
        elevation: lerp(m.elevation, v.mounts[j].elevation, w),
      })),
      deployment:
        u.deployment === null || v.deployment === null
          ? u.deployment
          : lerp(u.deployment, v.deployment, w),
    };
  });
  return { time: t, units, fallen: a.fallen };
}
