// What a second client needs to pose and draw a battle's units as this page
// does (the `native-visual-presentation` manifest's `units`,
// `web/src/battle/benchmark/unitManifest.ts`): the pose driver's feel, each
// fielded unit type's facts and the appearance sets `AppearanceCatalog`
// picks from, and each of those appearances' joints and clips or rig. The
// browser resolves everything static here; native ports the per-frame
// derivations (`poseDriver.ts`, `articulation.ts`, `clipTable.ts`).
import game from "@fixtures/game.json";
import type { SideFactions } from "@packages/scene-assets/src/appearanceCatalog";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import { articulationRig } from "@packages/scene-assets/src/articulation";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { Side, Socket } from "@packages/scene-assets/src/schema";
import {
  airborne,
  MOUNT_NODES,
  vehicleClass,
  type UnitCatalog,
} from "@packages/scene-assets/src/units";
import { buildClipTable } from "@packages/battle-renderer/src/models/clipTable";
import type { PoseFeel } from "@packages/battle-renderer/src/models/poseDriver";
import type {
  AppearanceMuzzle,
  AppearanceRecord,
  DrawnUnits,
  TrsRecord,
  UnitDraws,
  UnitTypeRecord,
} from "@web/battle/benchmark/unitManifest";

const SIDES: readonly Side[] = ["blue", "red"];

/** `words` as base64 of their little-endian bytes. */
function base64(floats: Float32Array): string {
  const bytes = new Uint8Array(floats.buffer, floats.byteOffset, floats.byteLength);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000)
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary);
}

/** The `units` section for a battle fielding `kinds`, drawn from `installed`
 *  under `units` for sides fighting for `factions`, posed by `pose` with
 *  soldiers holding a firing pose `leanHold` seconds. Every appearance the
 *  types can draw is described; skinned bodies sharing a clip table share
 *  one table. */
export function describeUnits({
  kinds,
  installed,
  units,
  factions,
  pose,
  leanHold,
}: {
  kinds: readonly string[];
  installed: InstalledAppearances;
  units: UnitCatalog;
  factions?: SideFactions;
  pose: PoseFeel;
  leanHold: number;
}): DrawnUnits {
  const catalog = new AppearanceCatalog(installed, units, factions);
  const look = (side: Side, names: readonly string[]) =>
    names
      .filter((n) => installed.appearances.has(n))
      .map((n) => {
        const faction = factions && installed.appearances.get(n)?.factions?.[factions[side]];
        return faction && installed.appearances.has(faction) ? faction : n;
      });
  const types: Record<string, UnitTypeRecord> = {};
  for (const kind of [...new Set(kinds)].sort()) {
    const type = units.type(kind);
    const hull = units.hull(kind);
    const cls = vehicleClass(type);
    const draws = (side: Side): UnitDraws => ({
      hull: hull ? look(side, [type.appearance ?? ""]) : [],
      slots: units.slots(kind).map((s) => look(side, units.soldier(s).appearance)),
      operated: type.mounts.map((m) =>
        m.operator_appearance
          ? {
              active: look(side, m.operator_appearance.active),
              carried: look(side, m.operator_appearance.carried),
            }
          : null,
      ),
    });
    types[kind] = {
      hull: hull
        ? { halfTrack: hull.half_extents_m[1] * ((cls ? pose.gauge[cls] : undefined) ?? 1) }
        : null,
      airborne: airborne(type),
      roles: [...catalog.mountRoles(kind)],
      mounts: type.mounts.map((m) => {
        const on = m.on === null ? -1 : type.mounts.findIndex((r) => r.id === m.on);
        return {
          weapon: m.weapons[0] ?? "default",
          on: on < 0 ? null : on,
          activePose: m.operator_appearance?.active_pose ?? null,
        };
      }),
      draws: { blue: draws("blue"), red: draws("red") },
    };
  }
  const drawn = new Set(
    Object.values(types).flatMap((t) =>
      SIDES.flatMap((side) => {
        const d = t.draws[side];
        return [
          ...d.hull,
          ...d.slots.flat(),
          ...d.operated.flatMap((o) => (o ? [...o.active, ...o.carried] : [])),
        ];
      }),
    ),
  );
  const appearances: Record<string, AppearanceRecord> = {};
  const tables: DrawnUnits["tables"] = [];
  const tableOf = new Map<string, string>();
  for (const name of [...drawn].sort()) {
    const bundle = installed.appearances.get(name)!.bundle;
    if (bundle.kind === "skinned") {
      const skeleton = installed.skeletons.get(bundle.skeleton);
      if (!skeleton)
        throw new Error(`appearance ${name}: skeleton ${bundle.skeleton} is not installed`);
      const table = buildClipTable(skeleton, bundle.joints);
      const encoded = base64(table.data);
      let tableName = tableOf.get(encoded);
      if (!tableName) {
        tableName = `clips-${tables.length}`;
        tableOf.set(encoded, tableName);
        tables.push({ name: tableName, base64: encoded });
      }
      appearances[name] = {
        kind: "skinned",
        joints: bundle.joints.map((j) => ({ name: j.name, parent: j.parent, bind: trs(j.bind) })),
        table: tableName,
        clips: Object.fromEntries(
          [...table.clips].map(([clip, c]) => [
            clip,
            {
              offset: c.offset,
              frames: c.frames,
              duration: c.duration,
              rate: c.rate,
              loop: c.loop,
              stride: c.stride_m,
            },
          ]),
        ),
        farPose: { ...bundle.far_pose },
        corpsePoses: bundle.corpse_poses.map((p) => ({ ...p })),
        muzzle: muzzleSocket(bundle.sockets),
      };
    } else if (bundle.kind === "articulated") {
      const rig = articulationRig(bundle.nodes);
      appearances[name] = {
        kind: "articulated",
        nodes: bundle.nodes.map((n) => ({ name: n.name, parent: n.parent, bind: trs(n.bind) })),
        rig: {
          turret: rig.turret,
          gun: rig.gun,
          hmg: rig.hmg,
          hmgGun: rig.hmgGun,
          pitch: { gun: [...rig.pitch.gun], hmg: [...rig.pitch.hmg] },
          stroke: Number.isFinite(rig.stroke) ? rig.stroke : null,
          wheels: rig.wheels.map((w) => ({ ...w })),
          tracks: rig.tracks.map((t) => ({ ...t })),
          rotors: rig.rotors.map((r) => ({
            node: r.node,
            radius: r.radius,
            blades: r.blades,
            chord: r.chord,
            rest: r.rest,
            mast_below: r.mast_below,
            mast_above: r.mast_above,
            mast_radius: r.mast_radius,
          })),
          deploy: rig.deploy.map((m) => ({
            node: m.node,
            start: m.start,
            end: m.end,
            move: [m.move[0], m.move[1], m.move[2]],
            turn: [m.turn[0], m.turn[1], m.turn[2]],
          })),
          muzzles: muzzleNodes(bundle.nodes),
        },
      };
    } else throw new Error(`appearance ${name} draws a unit but is a ${bundle.kind} bundle`);
  }
  const detail = game.presentation.models;
  return {
    pose: pose as unknown as Record<string, unknown>,
    leanHold,
    detail: {
      lod_px: [detail.lod_px[0], detail.lod_px[1], detail.lod_px[2]],
      impostor_px: detail.impostor_px,
    },
    tints: {
      blue: [installed.sides.blue[0], installed.sides.blue[1], installed.sides.blue[2]],
      red: [installed.sides.red[0], installed.sides.red[1], installed.sides.red[2]],
    },
    types,
    appearances,
    tables,
  };
}

const trs = (t: {
  t: ArrayLike<number>;
  r: ArrayLike<number>;
  s: ArrayLike<number>;
}): TrsRecord => ({
  t: [t.t[0], t.t[1], t.t[2]],
  r: [t.r[0], t.r[1], t.r[2], t.r[3]],
  s: [t.s[0], t.s[1], t.s[2]],
});

/** A body's weapon `muzzle` socket, as `DrawnModels.soldier` reads it; null
 *  where it has none. */
function muzzleSocket(sockets: readonly Socket[]): AppearanceMuzzle {
  const socket = sockets.find((k) => k.name === "muzzle");
  return socket ? { joint: socket.joint, offset: trs(socket.offset) } : null;
}

/** Each articulated mount's muzzle node by role (`MOUNT_NODES`), as
 *  `DrawnModels.vehicle` finds it by name, where the model has it. */
function muzzleNodes(nodes: readonly { name: string }[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const [role, { muzzle }] of Object.entries(MOUNT_NODES)) {
    const node = nodes.findIndex((n) => n.name === muzzle);
    if (node >= 0) out[role] = node;
  }
  return out;
}
