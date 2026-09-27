// A feed replay: synthetic observation frames for one unit, scripted in time,
// published at the simulation's tick rate and interpolated between ticks the
// way the battle interpolates presentation. It drives the real pose driver,
// so the workbench shows what a model does when the simulation walks, runs,
// fires, is pinned, dies, drives, turns, slews, and deploys.
//
// Soldiers move on their own paths (each his own offset, speed and delay),
// never as a formation, so the replay exercises per-soldier posing.

import { clamp, lerp } from "math";
import village from "@fixtures/village.json";
import type {
  FeedFrame,
  FeedMount,
  FeedUnit,
  UnitKindName,
} from "@packages/battle-renderer/src/models/poseDriver";

export const TICK_HZ = village.tick_hz;

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
export const TANK_BEATS: Beat[] = [
  { at: 0, name: "idle" },
  { at: 1, name: "drive" },
  { at: 5, name: "turn" },
  { at: 8, name: "slew" },
  { at: 11, name: "fire" },
  { at: 13, name: "hmg" },
  { at: 18, name: "end" },
];
export const SUPPLY_BEATS: Beat[] = [
  { at: 0, name: "idle" },
  { at: 1, name: "drive" },
  { at: 5, name: "deploy" },
  { at: 15, name: "deployed" },
  { at: 17, name: "pack" },
  { at: 24, name: "end" },
];

export const JEEP_BEATS: Beat[] = [
  { at: 0, name: "idle" },
  { at: 1, name: "drive" },
  { at: 4, name: "turn" },
  { at: 6, name: "slew" },
  { at: 8, name: "hmg" },
  { at: 14, name: "end" },
];

export function beatsFor(kind: UnitKindName): Beat[] {
  if (kind === "tank") return TANK_BEATS;
  if (kind === "supply") return SUPPLY_BEATS;
  if (kind === "jeep") return JEEP_BEATS;
  return INFANTRY_BEATS;
}

export function beatAt(kind: UnitKindName, t: number): string {
  const beats = beatsFor(kind);
  let name = beats[0].name;
  for (const b of beats) if (t >= b.at) name = b.name;
  return name;
}

export function replayLength(kind: UnitKindName): number {
  return beatsFor(kind).at(-1)!.at;
}

const ramp = (t: number, t0: number, t1: number) => clamp((t - t0) / (t1 - t0), 0, 1);

/** Distance a soldier has covered by `t`: walking 1.4 m/s, then running 4 m/s. */
function infantryPath(t: number, delay: number): number {
  const s = t - delay;
  const walk = Math.min(Math.max(0, s - 2), 4) * 1.4;
  const run = Math.min(Math.max(0, s - 6), 3) * 4;
  return walk + run;
}

/** The unit at an exact tick time. */
function unitAt(kind: UnitKindName, t: number, soldiers: number): FeedUnit {
  if (kind === "tank") {
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
      mounts: [cannon, hmg],
      deployment: null,
      suppression: 0,
    };
  }
  if (kind === "jeep") {
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
      mounts: [hmg],
      deployment: null,
      suppression: 0,
    };
  }
  if (kind === "supply") {
    const drive = Math.min(Math.max(0, t - 1), 4) * 4;
    const deployment = t < 17 ? ramp(t, 5, 15) : 1 - ramp(t, 17, 24);
    return {
      id: 1,
      kind,
      side: "blue",
      position: [drive, 0, 0],
      yaw: 0,
      soldiers: [],
      mounts: [],
      deployment,
      suppression: 0,
    };
  }
  // Infantry: each soldier on his own lane, speed profile and delay.
  const members = Array.from({ length: soldiers }, (_, i) => {
    const delay = i * 0.35;
    const lane = (i - (soldiers - 1) / 2) * 1.6;
    const along = infantryPath(t, delay) * (1 - i * 0.04);
    return { id: 100 + i, position: [along, lane, 0] as [number, number, number] };
  });
  const firing = t >= 9.2 && t < 12.5;
  return {
    id: 1,
    kind,
    side: "blue",
    position: members[0].position,
    yaw: 0,
    soldiers: members,
    mounts: [
      {
        bearing: 0.2,
        elevation: 0.02,
        shots: firing ? Math.floor((t - 9.2) / 0.6) * soldiers : t >= 12.5 ? 6 * soldiers : 0,
      },
    ],
    deployment: null,
    // Pinned at the rules' collapse level, then recovering.
    suppression: t >= 12.5 && t < 16 ? village.suppression.collapse_level : t >= 16 ? 0.4 : 0,
  };
}

/** The published frame at tick `k`, with the soldiers who have fallen. */
export function feedTick(kind: UnitKindName, k: number, soldiers = 4): FeedFrame {
  const t = k / TICK_HZ;
  const unit = unitAt(kind, t, soldiers);
  const fallen =
    kind !== "tank" && kind !== "supply" && kind !== "jeep" && t >= 16
      ? unit.soldiers.slice(0, 1).map((s) => ({
          soldier: s.id,
          position: s.position,
          yaw: 0.3,
          kind,
          side: "blue" as const,
        }))
      : [];
  if (fallen.length) unit.soldiers = unit.soldiers.filter((s) => s.id !== fallen[0].soldier);
  return { time: t, units: [unit], fallen };
}

/** The frame the presentation shows at `t`: positions and bearings eased
 *  between the two ticks around it, everything else from the earlier tick. */
export function feedAt(kind: UnitKindName, t: number, soldiers = 4): FeedFrame {
  const k = Math.floor(t * TICK_HZ);
  const a = feedTick(kind, k, soldiers);
  const b = feedTick(kind, k + 1, soldiers);
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
