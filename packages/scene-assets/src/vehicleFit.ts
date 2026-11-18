// A vehicle's fit to the unit types that draw it: its running gear, each
// mount by the rig its model declares, a deploy capability's legs and mast,
// the hull box with its dressing, and each part's hardware, all from the
// type's own resolved numbers. The validator (`validate.ts`) runs it on every
// articulated appearance.

import { quat, vec3, type Vec3 } from "math";
import { pointAt } from "./trs.ts";
import { mountMuzzles, muzzleOffset, type MountMuzzle } from "./mountMuzzle.ts";
import {
  MOUNT_NODES,
  hullFixed,
  isArticulation,
  mountRoles,
  type Articulation,
  type MountDraws,
  type UnitCatalog,
  type UnitType,
} from "./units.ts";
import {
  DEPLOY_EXTRAS,
  REST_ARTICULATION,
  articulate,
  articulationRig,
  isRotor,
  restLocals,
} from "./articulation.ts";
import {
  articulatedPositions,
  articulatedWorlds,
  positionsBounds,
  worldTransforms,
} from "./pose.ts";
import { unitArtRule, type DressingAllowance } from "./unitArt.ts";
import { extentFindings } from "./boxFit.ts";
import type { ArticulatedNode, Finding, Tolerances } from "./schema.ts";
import { finding, fmt } from "./finding.ts";

const MAST_CHAIN = ["deploy_mast", "deploy_mast_2", "deploy_mast_3", "deploy_mast_head"];
const ARC_BEARINGS = 12;
/** The node a mount role's parts hang from that the hull box leaves out: the
 *  gun's barrel overhangs the hull's front; the roof HMG and its ring stand
 *  above the roof. */
const OFF_HULL: Record<Articulation, string> = {
  gun: MOUNT_NODES.gun.pitch,
  hmg: MOUNT_NODES.hmg.yaw,
};

/** A node whose parts are dressing (antennas, stowage, crew): measured
 *  against its class's dressing allowance (`unitArt.ts`), not the hull's
 *  tolerance. Node extras keep numbers only, so the name is the mark. */
const DRESSING = "dressing_";
const isDressing = (name: string) => name.startsWith(DRESSING);

/**
 * One dressing node's parts, at rest, against the hull box of half extents
 * `half` standing on the ground: within `bulky_m` of every face, and above
 * that only thin (no wider than `thin_m` either way, as an antenna is) and
 * no higher than `thin_top_m` over the box's top, so dressing never reads
 * as cover the simulation lacks.
 */
function dressingFindings(
  label: string,
  positions: Float32Array,
  half: Vec3,
  { bulky_m, thin_m, thin_top_m }: DressingAllowance,
): Finding[] {
  if (!positions.length) return [];
  const b = positionsBounds(positions);
  const top = 2 * half[2];
  const faces: [string, number, number][] = [
    ["-x", -b.min[0], half[0]],
    ["+x", b.max[0], half[0]],
    ["-y", -b.min[1], half[1]],
    ["+y", b.max[1], half[1]],
  ];
  const over = faces
    .filter(([, reach, box]) => reach > box + bulky_m)
    .map(([face, reach, box]) => `${face} ${fmt(reach - box)} m past the box`);
  // What rises above the bulky allowance must be thin, and not too tall.
  const high = { min: [Infinity, Infinity], max: [-Infinity, -Infinity] };
  for (let i = 0; i < positions.length; i += 3)
    if (positions[i + 2] > top + bulky_m)
      for (const k of [0, 1]) {
        high.min[k] = Math.min(high.min[k], positions[i + k]);
        high.max[k] = Math.max(high.max[k], positions[i + k]);
      }
  const across = Math.max(high.max[0] - high.min[0], high.max[1] - high.min[1]);
  if (across > thin_m)
    over.push(`+z ${fmt(b.max[2] - top)} m over the box's top, ${fmt(across)} m across`);
  else if (b.max[2] > top + thin_top_m) over.push(`+z ${fmt(b.max[2] - top)} m over the box's top`);
  return over.length
    ? [
        finding(
          "fit.dressing",
          `${label} reaches ${over.join("; ")} (dressing: ${bulky_m} m past any face, and above that only parts at most ${thin_m} m across, up to ${thin_top_m} m)`,
          "bring bulky dressing (stowage, crew, tarps) within its allowance of the hull box, and give each tall thin part (an antenna) its own dressing node",
        ),
      ]
    : [];
}
/** Whether `name` matches a part's node pattern: exact, or a prefix before a trailing `*`. */
const matchesNode = (pattern: string, name: string) =>
  pattern.endsWith("*") ? name.startsWith(pattern.slice(0, -1)) : name === pattern;

/**
 * An articulated model against one unit type that draws it, from the type's
 * own resolved numbers: its running gear (tracks or wheels), each mount by
 * the rig the model declares for it (`draws`, the catalog entry's `mounts`:
 * every mount declared once, its rig's nodes present and chained, the muzzle
 * where the type's mount row puts it at rest and through every bearing), a
 * deploy capability's legs and mast, the hull box, and each of its parts'
 * hardware.
 */
export function typeFindings(
  path: string,
  nodes: ArticulatedNode[],
  units: UnitCatalog,
  id: string,
  tolerances: Tolerances,
  draws: MountDraws | null,
): Finding[] {
  const label = `${path} (as ${id})`;
  const type = units.type(id);
  const hull = units.hull(id);
  const out: Finding[] = [];
  if (!hull) {
    out.push(
      finding(
        "fit.type_appearance",
        `${label}: unit type ${id} is a squad; its soldiers draw soldier appearances, not a vehicle`,
        `name this appearance only from a hull type's appearance`,
      ),
    );
    return out;
  }
  const index = new Map(nodes.map((n, i) => [n.name, i]));
  const isUnder = (child: number, ancestor: number) => {
    for (let p = nodes[child].parent; p >= 0; p = nodes[p].parent) if (p === ancestor) return true;
    return false;
  };
  const missing = (name: string) =>
    out.push(
      finding(
        "nodes.missing",
        `${label}: no "${name}" node`,
        `add an empty named "${name}" at the part's pivot`,
      ),
    );
  const chain = (parent: string, child: string) => {
    const [p, c] = [index.get(parent), index.get(child)];
    if (p !== undefined && c !== undefined && !isUnder(c, p))
      out.push(
        finding(
          "nodes.hierarchy",
          `${label}: "${child}" is not under "${parent}"`,
          `parent "${child}" to "${parent}"`,
        ),
      );
  };
  const up = (name: string) => {
    const node = nodes[index.get(name) ?? -1];
    if (node && node.pivot[2] <= 0)
      out.push(
        finding(
          "basis.up",
          `${label}: ${name} pivot at z ${fmt(node.pivot[2])} m`,
          "export with +Y up in glTF; the bake turns it to Z up",
        ),
      );
  };
  const worlds = articulatedWorlds(nodes);

  // Running gear.
  if ("tracked" in type.mobility) {
    for (const name of ["track_L", "track_R"]) {
      const node = nodes[index.get(name) ?? -1];
      if (!node) missing(name);
      else if (!(node.extras.track_length_m > 0 && node.extras.link_pitch_m > 0))
        out.push(
          finding(
            "nodes.track_properties",
            `${label}: "${name}" lacks positive track_length_m and link_pitch_m`,
            `set custom properties track_length_m (loop length) and link_pitch_m on "${name}" and export with custom properties`,
          ),
        );
    }
  } else if ("wheeled" in type.mobility) {
    if (!nodes.some((n) => n.name.startsWith("wheel_"))) missing("wheel_*");
    // Front wheels (wheel_F*) ahead of the rear ones (wheel_R*): the hull faces +X.
    const wheelX = (row: string) =>
      nodes.filter((n) => n.name.startsWith(`wheel_${row}`)).map((n) => n.pivot[0]);
    const [front, rear] = [wheelX("F"), wheelX("R")];
    if (front.length && rear.length && Math.min(...front) <= Math.max(...rear))
      out.push(
        finding(
          "basis.forward",
          `${label}: front wheels (wheel_F*) are not ahead of rear wheels (wheel_R*) along +X`,
          `face the ${type.name} along +X after the catalog's basis_yaw_deg`,
        ),
      );
  }

  // Mounts, by the rig the model declares for each.
  out.push(...mountDrawFindings(label, type, draws));
  const roles = mountRoles(type, draws).map((role) => (isArticulation(role) ? role : "hand"));
  const drawn: (MountNodes | null)[] = roles.map((role) =>
    role === "hand" ? null : MOUNT_NODES[role],
  );
  for (const [i, role] of roles.entries()) {
    if (role === "hand") continue;
    const n = MOUNT_NODES[role];
    for (const name of [n.yaw, n.pitch, n.muzzle]) if (!index.has(name)) missing(name);
    chain(n.yaw, n.pitch);
    chain(n.pitch, n.muzzle);
    up(n.yaw);
    if (role === "gun") {
      const muzzle = index.get(n.muzzle);
      const muzzleX = muzzle === undefined ? 1 : pointAt(worlds[muzzle], [0, 0, 0])[0];
      const yaw = index.get(n.yaw);
      const pivotX = yaw === undefined ? 0 : pointAt(worlds[yaw], [0, 0, 0])[0];
      if (muzzleX <= pivotX)
        out.push(
          finding(
            "basis.forward",
            `${label}: ${type.mounts[i].name}'s muzzle at x ${fmt(muzzleX)} m, behind its mount pivot at x ${fmt(pivotX)} m`,
            "face the hull along +X after the catalog's basis_yaw_deg",
          ),
        );
    }
  }
  out.push(
    ...muzzleFindings(label, nodes, worlds, index, drawn, tolerances, {
      name: `units.${id}.mounts`,
      mounts: mountMuzzles(type.mounts),
    }),
  );

  // Stationary service does not imply legs or a mast. Declared deployment
  // hardware retains its chain and motion checks.
  const declaresDeployment = nodes.some(
    (node) =>
      node.name.startsWith("deploy_") ||
      DEPLOY_EXTRAS.some((key) => node.extras?.[key] !== undefined),
  );
  if (type.capabilities.deploy && declaresDeployment) {
    const legs = nodes
      .map((n) => n.name.match(/^deploy_leg_([A-Za-z0-9]+)(?:_(?:jack|pad))?$/)?.[1])
      .filter((leg): leg is string => !!leg);
    for (const leg of new Set(legs)) {
      if (!index.has(`deploy_leg_${leg}`)) missing(`deploy_leg_${leg}`);
      for (const suffix of ["_jack", "_pad"])
        if (!index.has(`deploy_leg_${leg}${suffix}`)) missing(`deploy_leg_${leg}${suffix}`);
      chain(`deploy_leg_${leg}`, `deploy_leg_${leg}_jack`);
      chain(`deploy_leg_${leg}_jack`, `deploy_leg_${leg}_pad`);
    }
    if (MAST_CHAIN.some((name) => index.has(name))) {
      for (const name of MAST_CHAIN) if (!index.has(name)) missing(name);
      for (let i = 1; i < MAST_CHAIN.length; i++) chain(MAST_CHAIN[i - 1], MAST_CHAIN[i]);
      up("deploy_mast");
    }
    out.push(...deployFindings(label, nodes, index, tolerances));
  }

  // The hull box, without what its mounts carry beyond it, its dressing or
  // its rotors (a rotor's disc is not its airframe's size). Dressing nested
  // in dressing is measured with the outer node.
  const dressing = nodes.flatMap((n, i) =>
    isDressing(n.name) && !nodes.some((d, j) => isDressing(d.name) && isUnder(i, j)) ? [i] : [],
  );
  const excluded = roles
    .flatMap((role) => (role === "hand" ? [] : [index.get(OFF_HULL[role])]))
    .filter((i): i is number => i !== undefined)
    .concat(dressing)
    .concat(nodes.flatMap((n, i) => (isRotor(n.name) ? [i] : [])));
  const hullPositions = articulatedPositions(
    nodes,
    worlds,
    0,
    (i) => !excluded.some((e) => i === e || isUnder(i, e)),
  );
  out.push(
    ...extentFindings(label, hullPositions, hull.half_extents_m, {
      code: "fit.hull_extents",
      rule: `units.${id}.body.hull.half_extents_m [${hull.half_extents_m.join(", ")}]`,
      tolerance: tolerances.hull_extent_m,
      fix: `fit the hull to the simulation's box; parts that stand outside it (antennas, stowage, crew) go under a "${DRESSING}*" node, held to its class's dressing allowance`,
    }),
  );
  const allowance = unitArtRule(type.vehicle_class).dressing;
  for (const d of dressing)
    out.push(
      ...dressingFindings(
        `${label}: "${nodes[d].name}"`,
        articulatedPositions(nodes, worlds, 0, (i) => i === d || isUnder(i, d)),
        hull.half_extents_m,
        allowance,
      ),
    );

  // Each part's hardware.
  for (const part of type.parts ?? []) {
    const patterns = units.view.parts[part]?.nodes ?? [];
    const absent = patterns.filter((p) => !nodes.some((n) => matchesNode(p, n.name)));
    if (absent.length)
      out.push(
        finding(
          "fit.part_nodes",
          `${label}: lists part ${part}, but the model draws no ${absent.map((p) => `"${p}"`).join(", ")} node`,
          `a type listing part ${part} must draw its hardware: model it and name its nodes ${patterns.join(", ")}`,
        ),
      );
  }
  return out;
}

/** A hull model's mount declarations against its type: every mount with a
 *  muzzle is declared, each once, by a rig the renderer has or by the hull
 *  (a turning mount its model has no rig for), and nothing names a mount the
 *  type lacks. */
function mountDrawFindings(label: string, type: UnitType, draws: MountDraws | null): Finding[] {
  const out: Finding[] = [];
  const fix = (m: string) =>
    `declare it in the appearance's catalog entry: "mounts": { "${m}": ${[
      ...Object.keys(MOUNT_NODES),
      "hull",
    ]
      .map((a) => `"${a}"`)
      .join(" | ")} }`;
  const names = new Set(type.mounts.map((m) => m.id));
  const rigs = new Map<string, string>();
  for (const [mount, rig] of Object.entries(draws ?? {})) {
    if (!names.has(mount))
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: declares mount "${mount}", which ${type.id} does not have (${[...names].join(", ") || "none"})`,
          "name the type's mount by its catalog row's id",
        ),
      );
    if (rig === "hull") continue;
    if (!isArticulation(rig))
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: mount "${mount}" is drawn by "${rig}", which is not a rig`,
          fix(mount),
        ),
      );
    else if (rigs.has(rig))
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: mounts "${rigs.get(rig)}" and "${mount}" are both drawn by the ${rig} rig`,
          "give each mount its own rig; two mounts never share a muzzle",
        ),
      );
    else rigs.set(rig, mount);
  }
  // A mount fixed in the hull is drawn by it; every mount that turns needs a rig.
  for (const m of type.mounts)
    if (m.muzzle_m && !draws?.[m.id] && !hullFixed(m))
      out.push(
        finding("fit.mount_draw", `${label}: mount "${m.id}" is not drawn by any rig`, fix(m.id)),
      );
  return out;
}

/** The nodes that draw a mount: the one it yaws on and its muzzle. */
interface MountNodes {
  yaw: string;
  muzzle: string;
}

/** Each mount's drawn muzzle against the simulation's (`mountMuzzle.ts`): at
 *  rest on its row, and as its carrier (the turret it is on) and its own
 *  ring turn through every pair of bearings, where the simulation's model
 *  puts it. The drawn mount's yaw is relative to its carrier, whose yaw it
 *  adds to its world bearing. */
function muzzleFindings(
  label: string,
  nodes: ArticulatedNode[],
  worlds: ReturnType<typeof articulatedWorlds>,
  index: Map<string, number>,
  /** Each mount's nodes, parallel to `rule.mounts`; null for a hand weapon. */
  drawn: (MountNodes | null)[],
  tolerances: Tolerances,
  rule: { name: string; mounts: (MountMuzzle | null)[] },
): Finding[] {
  const out: Finding[] = [];
  const yawed = (yaws: Map<number, number>) => {
    const overrides: Record<string, ArticulatedNode["bind"]> = {};
    for (const [node, bearing] of yaws) {
      const bind = nodes[node].bind;
      const r = quat.setAxisAngle(quat.create(), [0, 0, 1], bearing);
      overrides[nodes[node].name] = { ...bind, r: quat.multiply(quat.create(), r, bind.r) };
    }
    return articulatedWorlds(nodes, overrides);
  };
  const expected = vec3.create();
  drawn.forEach((mount, i) => {
    const m = rule.mounts[i];
    if (!mount) return;
    const [yaw, muzzle] = [index.get(mount.yaw), index.get(mount.muzzle)];
    if (!m || muzzle === undefined) return;
    const what = `${rule.name}[${i}] (${m.id})`;
    const rest = pointAt(worlds[muzzle], [0, 0, 0]);
    muzzleOffset(expected, m, 0, 0, 0);
    const error = vec3.distance(rest, expected);
    if (error > tolerances.vehicle_muzzle_m)
      out.push(
        finding(
          "fit.vehicle_muzzle",
          `${label}: ${mount.muzzle} at [${rest.map(fmt).join(", ")}], ${what} puts it at [${[...expected].map(fmt).join(", ")}] (${fmt(error)} m off, tolerance ${tolerances.vehicle_muzzle_m})`,
          "move the gun's muzzle to the row's pivot plus muzzle, or change the type's mount row in the unit catalog",
        ),
      );
    if (yaw === undefined) return;
    const carrierNodes = m.on === null ? null : drawn[m.on];
    const carrier = carrierNodes ? index.get(carrierNodes.yaw) : undefined;
    const carriedBearings = carrier === undefined ? 1 : ARC_BEARINGS;
    let worst = 0;
    for (let c = 0; c < carriedBearings; c++) {
      const carried = (2 * Math.PI * c) / ARC_BEARINGS;
      for (let k = 0; k < ARC_BEARINGS; k++) {
        const own = (2 * Math.PI * k) / ARC_BEARINGS;
        const yaws = new Map([[yaw, own]]);
        if (carrier !== undefined) yaws.set(carrier, carried);
        const at = pointAt(yawed(yaws)[muzzle], [0, 0, 0]);
        muzzleOffset(expected, m, 0, carried, carried + own);
        worst = Math.max(worst, vec3.distance(at, expected));
      }
    }
    if (worst > tolerances.muzzle_arc_m)
      out.push(
        finding(
          "fit.muzzle_arc",
          `${label}: as ${mount.yaw} turns, ${mount.muzzle} leaves the arc ${what} swings it through by up to ${fmt(worst)} m`,
          `put the ${mount.yaw}'s yaw pivot on the row's pivot_m, under its carrier's ring as the simulation's muzzle model turns it`,
        ),
      );
  });
  return out;
}

/** Deploying parts must be authored to move, and the legs' pads must reach
 *  the ground when deployed. */
function deployFindings(
  label: string,
  nodes: ArticulatedNode[],
  index: Map<string, number>,
  tolerances: Tolerances,
): Finding[] {
  const rig = articulationRig(nodes);
  if (!rig.deploy.length)
    return [
      finding(
        "nodes.deploy_motion",
        `${label}: no node carries a deploy window (deploy_start, deploy_end)`,
        `set custom properties ${DEPLOY_EXTRAS.join(", ")} on each deploying part (legs, jacks, mast stages)`,
      ),
    ];
  const worlds = worldTransforms(
    nodes.map((n) => n.parent),
    articulate(restLocals(nodes), nodes, rig, { ...REST_ARTICULATION, deploy: 1 }),
  );
  const out: Finding[] = [];
  for (const [name, pad] of index) {
    if (!/^deploy_leg_[A-Za-z0-9]+_pad$/.test(name)) continue;
    const under = (i: number) => {
      for (let p = i; p >= 0; p = nodes[p].parent) if (p === pad) return true;
      return false;
    };
    const positions = articulatedPositions(nodes, worlds, 0, under);
    if (!positions.length) continue;
    const low = positionsBounds(positions).min[2];
    if (Math.abs(low) > tolerances.ground_m)
      out.push(
        finding(
          "nodes.deploy_motion",
          `${label}: deployed, "${name}" rests at z ${fmt(low)} m, not on the ground (± ${tolerances.ground_m})`,
          "adjust the jack's deploy_move_z so the pad meets z = 0 at deploy 1",
        ),
      );
  }
  return out;
}
