// Where a jet's engines and wings are, read from its source art: the facts its
// exhaust effects are drawn from (`presentation.effects.jet_exhaust.airframes`,
// crates/presentation/src/effects/exhaust.rs). The runtime bundle merges a jet's
// static parts into one hull, so nothing in it still says which vertices are a
// nozzle or a wing; the source GLB's named nodes do, and this reads them once.
//
// Everything is in engine space (the bake's frame: Z up, +X forward, +Y left,
// metres, origin on the ground), read from each node's LOD0 mesh only:
// - a nozzle per exhaust opening, a node named `<part>_bore` or `<part>_exhaust`:
//   its exit plane is the rearmost vertex of any part inside the opening's
//   outline behind it that still walls the exhaust in above and below (lip,
//   petals, a 2-D nozzle's flaps; not a tailplane root overhanging one side),
//   its middle the opening's, and its radius the circle of the opening's area
//   (a round or a 2-D nozzle alike);
// - a wingtip per wing (`wing_L`, `wing_R`): the middle of its outermost
//   section (`TIP_SHARE` of its span), out at its full span. That is the real
//   tip chord's middle, so a cropped-delta or straight tip (Typhoon, Super
//   Hornet) trails from mid-chord, not from its leading corner;
// - each wing's planform: its sections at evenly spaced spans from its root
//   (its longest chord, where it meets the body; the outer of the pair's two
//   roots, so the pair mirrors) to its tip, each a leading and trailing edge
//   and the top of its upper surface.

import type { Vec3 } from "math";
import { importScene, type Scene, type SceneNode } from "./scene.ts";
import { pointAt } from "./trs.ts";

/** One wing section, at `span_y` across the jet. */
export interface WingStation {
  span_y: number;
  leading_x: number;
  trailing_x: number;
  top_z: number;
}

/** The derived geometry of one jet appearance, as the presentation reads it. */
export interface JetGeometry {
  nozzles: { at: Vec3; radius_m: number }[];
  wingtips: { at: Vec3 }[];
  wings: { stations: WingStation[] }[];
}

/** The sections each wing's planform is read at, root and tip included. */
export const WING_STATIONS = 6;
/** The share of a wing's span, root to tip, its outermost section is read at:
 *  inside the rounded or capped tip, whose last few centimetres art often
 *  closes with slivers that are no part of the planform (one side of a pair
 *  but not the other). The wingtip is that section's middle, out at the
 *  wing's full span. */
const TIP_SHARE = 0.97;
/** Likewise its innermost section is read just outside its root, clear of the
 *  fairings and gaps the art sinks into the body there. */
const ROOT_SHARE = 0.03;
/** The sections a wing is scanned at for its root, and how near its longest
 *  chord (a share) the innermost section must come to be it. */
const ROOT_SCAN = 64;
const ROOT_CHORD = 0.01;
/** Every derived number is kept to the centimetre: finer than any effect needs,
 *  and coarse enough that a float's last bit never changes the fixture. */
const round = (v: number) => Math.round(v * 100) / 100 + 0;

const LOD = /_LOD(\d+)$/;
const EXHAUST = /^(.+)_(bore|exhaust)$/;
const WINGS = ["wing_L", "wing_R"] as const;

/** A node's name without its `_LOD<n>` tier. */
const partName = (node: SceneNode) => node.name.replace(LOD, "");

/** The mesh nodes the art draws closest: LOD0, or untiered. */
function nearestTier(scene: Scene): SceneNode[] {
  return scene.nodes.filter((n) => {
    if (n.mesh === null || !n.live) return false;
    const tier = n.name.match(LOD);
    return !tier || tier[1] === "0";
  });
}

/** A node's vertices in engine space. */
function vertices(scene: Scene, node: SceneNode): Vec3[] {
  const out: Vec3[] = [];
  for (const p of scene.meshes[node.mesh!].primitives)
    for (let i = 0; i < p.positions.length; i += 3)
      out.push(pointAt(node.world, [p.positions[i], p.positions[i + 1], p.positions[i + 2]]));
  return out;
}

/** A node's triangles in engine space. */
function triangles(scene: Scene, node: SceneNode): [Vec3, Vec3, Vec3][] {
  const out: [Vec3, Vec3, Vec3][] = [];
  for (const p of scene.meshes[node.mesh!].primitives) {
    const at = (i: number): Vec3 =>
      pointAt(node.world, [p.positions[3 * i], p.positions[3 * i + 1], p.positions[3 * i + 2]]);
    for (let i = 0; i + 2 < p.indices.length; i += 3)
      out.push([at(p.indices[i]), at(p.indices[i + 1]), at(p.indices[i + 2])]);
  }
  return out;
}

/** The convex hull of points in a plane, counter-clockwise (monotone chain). */
function convexHull(points: [number, number][]): [number, number][] {
  const p = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  if (p.length < 3) return p;
  const cross = (o: number[], a: number[], b: number[]) =>
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list: [number, number][]) => {
    const h: [number, number][] = [];
    for (const q of list) {
      while (h.length >= 2 && cross(h[h.length - 2], h[h.length - 1], q) <= 0) h.pop();
      h.push(q);
    }
    return h.slice(0, -1);
  };
  return [...half(p), ...half([...p].reverse())];
}

/** A convex polygon's area. */
function area(hull: [number, number][]): number {
  let twice = 0;
  for (let i = 0; i < hull.length; i++) {
    const [a, b] = [hull[i], hull[(i + 1) % hull.length]];
    twice += a[0] * b[1] - b[0] * a[1];
  }
  return Math.abs(twice) / 2;
}

/** How far outside a counter-clockwise convex polygon a point may lie and
 *  still count as inside it, m: the art's rounding. */
const INSIDE_M = 0.02;
/** How far apart along the jet a nozzle's walls above and below its middle
 *  may be and still be one exit, m. */
const WALL_M = 0.15;

/** The point lies inside the counter-clockwise convex polygon (within `INSIDE_M`). */
function within(hull: [number, number][], [u, v]: [number, number]): boolean {
  return hull.every((a, i) => {
    const b = hull[(i + 1) % hull.length];
    const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    return (dx * (v - a[1]) - dy * (u - a[0])) / Math.hypot(dx, dy) >= -INSIDE_M;
  });
}

const extent = (values: number[]) => [Math.min(...values), Math.max(...values)];

/** Each exhaust opening's nozzle, in node order. Its exit is the rearmost
 *  vertex of any part that lies inside the opening (its outline seen from
 *  behind) and behind it: a round nozzle's lip and petals, or a 2-D nozzle's
 *  flaps, wherever the art modelled them (in the opening's own family or in
 *  the fuselage), so long as it still walls the exhaust in above and below. */
function nozzles(scene: Scene, parts: SceneNode[]): JetGeometry["nozzles"] {
  const everything = parts.flatMap((n) => vertices(scene, n));
  return parts.flatMap((node) => {
    if (!EXHAUST.test(partName(node))) return [];
    const opening = vertices(scene, node);
    const outline = convexHull(opening.map((p) => [p[1], p[2]]));
    const [y0, y1] = extent(opening.map((p) => p[1]));
    const [z0, z1] = extent(opening.map((p) => p[2]));
    const [x0] = extent(opening.map((p) => p[0]));
    const middle = (z0 + z1) / 2;
    const behind = everything.filter((p) => p[0] <= x0 && within(outline, [p[1], p[2]]));
    // A wall the exhaust still runs between: above and below its middle at
    // about the same station, not one overhang (a tailplane root) on one side.
    const walled = behind.filter((p) =>
      behind.some((q) => Math.abs(q[0] - p[0]) <= WALL_M && (q[2] - middle) * (p[2] - middle) < 0),
    );
    const exit = Math.min(x0, ...walled.map((p) => p[0]));
    return [
      {
        at: [round(exit), round((y0 + y1) / 2), round((z0 + z1) / 2)] as Vec3,
        radius_m: round(Math.sqrt(area(outline) / Math.PI)),
      },
    ];
  });
}

/** Where the plane `y = at` cuts a mesh's triangles. */
function section(tris: [Vec3, Vec3, Vec3][], at: number): Vec3[] {
  const cut: Vec3[] = [];
  for (const tri of tris)
    for (let i = 0; i < 3; i++) {
      const [a, b] = [tri[i], tri[(i + 1) % 3]];
      if ((a[1] - at) * (b[1] - at) > 0 || a[1] === b[1]) continue;
      const f = (at - a[1]) / (b[1] - a[1]);
      cut.push([a[0] + (b[0] - a[0]) * f, at, a[2] + (b[2] - a[2]) * f]);
    }
  return cut;
}

/** A wing's tip and its planform, root to tip. Its root is where its chord
 *  is longest, which is where it meets the body: an art wing often reaches
 *  into the fuselage with slivers that are no part of its planform. */
function wingCutter(scene: Scene, node: SceneNode, path: string) {
  const points = vertices(scene, node);
  // Its outward way: +Y for the left wing, −Y for the right.
  const out = Math.sign(points.reduce((s, p) => s + p[1], 0)) || 1;
  const [inner, tip] = extent(points.map((p) => p[1] * out));
  const tris = triangles(scene, node);
  const cutAt = (span: number) => {
    const y = span * out;
    const cut = section(tris, y);
    if (!cut.length) throw new Error(`${path}: ${partName(node)} has no section at y ${y}`);
    const [x0, x1] = extent(cut.map((p) => p[0]));
    return { y, x0, x1, top: Math.max(...cut.map((p) => p[2])) };
  };
  // Just inside its ends: the very end faces lie in their planes.
  const inside = (share: number, from: number) =>
    from + (tip - from) * Math.min(Math.max(share, 0.005), 0.995);
  const scan = Array.from({ length: ROOT_SCAN }, (_, k) =>
    cutAt(inside(k / (ROOT_SCAN - 1), inner)),
  );
  const longest = Math.max(...scan.map((c) => c.x1 - c.x0));
  const root = scan.find((c) => c.x1 - c.x0 >= longest * (1 - ROOT_CHORD))!.y * out;
  return { root, tip, out, tris, cutAt };
}

/** A wing's tip and planform from `root` (its span from the jet's middle, m)
 *  to its tip. */
function wing({ tip, out, tris, cutAt }: ReturnType<typeof wingCutter>, root: number) {
  const stations = Array.from({ length: WING_STATIONS }, (_, k): WingStation => {
    const share = ROOT_SHARE + (k / (WING_STATIONS - 1)) * (TIP_SHARE - ROOT_SHARE);
    const c = cutAt(root + (tip - root) * share);
    return {
      span_y: round(c.y),
      leading_x: round(c.x1),
      trailing_x: round(c.x0),
      top_z: round(c.top),
    };
  });
  const end = cutAt(root + (tip - root) * TIP_SHARE);
  const cut = section(tris, end.y);
  const [zLow, zHigh] = extent(cut.map((p) => p[2]));
  const at: Vec3 = [round((end.x0 + end.x1) / 2), round(tip * out), round((zLow + zHigh) / 2)];
  return { tip: { at }, stations };
}

/** One jet's geometry from its source GLB, framed as the bake frames it
 *  (`basis_yaw_deg`). Fails when the art names no exhaust or no wing. */
export function jetGeometry(bytes: Uint8Array, path: string, yawDeg: number): JetGeometry {
  const { scene, findings } = importScene(bytes, path, yawDeg);
  if (!scene) throw new Error(findings.map((f) => `${f.message} (${f.fix})`).join("\n") || path);
  const parts = nearestTier(scene);
  const found = nozzles(scene, parts);
  if (!found.length) throw new Error(`${path}: no exhaust node (<part>_bore or <part>_exhaust)`);
  const cutters = WINGS.map((name) => {
    const node = parts.find((n) => partName(n) === name);
    if (!node) throw new Error(`${path}: no ${name} node`);
    return wingCutter(scene, node, path);
  });
  // Both wings start at the same span, the outer of their two roots, so a
  // pair's planforms mirror each other however differently the art sank each
  // root into the body.
  const root = Math.max(...cutters.map((c) => c.root));
  const wings = cutters.map((c) => wing(c, root));
  return {
    nozzles: found,
    wingtips: wings.map((w) => w.tip),
    wings: wings.map((w) => ({ stations: w.stations })),
  };
}

/** The roster profile every fixed-wing jet extends. */
export const FIXED_WING_PROFILE = "roster_profile_fixed_wing";

/** The appearance of every fixed-wing jet in the roster's unit documents, sorted. */
export function fixedWingAppearances(
  documents: { units?: Record<string, { extends?: string; appearance?: string }> }[],
): string[] {
  const found = new Set<string>();
  for (const doc of documents)
    for (const [id, unit] of Object.entries(doc.units ?? {})) {
      if (unit.extends !== FIXED_WING_PROFILE) continue;
      if (!unit.appearance) throw new Error(`${id}: a fixed-wing jet without an appearance`);
      found.add(unit.appearance);
    }
  return [...found].sort();
}

/** An airframe as `presentation.json` keeps it: its engine kind, authored by
 *  hand, and the geometry derived here. */
export type Airframe = { engine: string } & JetGeometry;

/** `value` on one line, as the fixtures write short objects and arrays. */
function inline(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(inline).join(", ")}]`;
  if (value && typeof value === "object")
    return `{ ${Object.entries(value)
      .map(([k, v]) => `${JSON.stringify(k)}: ${inline(v)}`)
      .join(", ")} }`;
  return JSON.stringify(value);
}

/** `airframes`' text at the indent its key sits at. */
function airframesText(airframes: Record<string, Airframe>, pad: string): string {
  const i = (n: number) => pad + "  ".repeat(n);
  const rows = Object.entries(airframes).map(([name, a]) =>
    [
      `${i(1)}${JSON.stringify(name)}: {`,
      `${i(2)}"engine": ${JSON.stringify(a.engine)},`,
      `${i(2)}"nozzles": ${inline(a.nozzles)},`,
      `${i(2)}"wingtips": ${inline(a.wingtips)},`,
      `${i(2)}"wings": [`,
      a.wings
        .map((w) =>
          [
            `${i(3)}{`,
            `${i(4)}"stations": [`,
            w.stations.map((s) => `${i(5)}${inline(s)}`).join(",\n"),
            `${i(4)}]`,
            `${i(3)}}`,
          ].join("\n"),
        )
        .join(",\n"),
      `${i(2)}]`,
      `${i(1)}}`,
    ].join("\n"),
  );
  return `{\n${rows.join(",\n")}\n${pad}}`;
}

/** `presentation.json`'s text with `jet_exhaust.airframes` replaced by one
 *  airframe per appearance of `derived`, each keeping the engine kind it was
 *  authored with; everything else in the file is left byte for byte. */
export function withAirframes(text: string, derived: Record<string, JetGeometry>): string {
  const authored: Record<string, { engine?: string }> =
    JSON.parse(text).effects?.jet_exhaust?.airframes ?? {};
  const airframes: Record<string, Airframe> = {};
  for (const name of Object.keys(derived).sort()) {
    const engine = authored[name]?.engine;
    if (!engine)
      throw new Error(
        `presentation.effects.jet_exhaust.airframes.${name}: author its "engine" (an engines look; one without a flame never burns)`,
      );
    airframes[name] = { engine, ...derived[name] };
  }
  const key = text.match(/^([ \t]*)"airframes": \{/m);
  if (!key || text.indexOf('"airframes":', key.index! + key[0].length) !== -1)
    throw new Error('presentation.json: expected one "airframes" key');
  const open = key.index! + key[0].length - 1;
  let depth = 0;
  let close = open;
  for (; close < text.length; close++) {
    const c = text[close];
    if (c === '"') close = text.indexOf('"', close + 1);
    else if (c === "{") depth++;
    else if (c === "}" && --depth === 0) break;
  }
  return text.slice(0, open) + airframesText(airframes, key[1]) + text.slice(close + 1);
}

/** The roster's unit documents. */
export const ROSTER_DIR = "fixtures/units/roster";

/** Every fixed-wing roster jet's geometry by appearance, read through `read`
 *  (a repository path's bytes) and `list` (a directory's file names): the
 *  roster, the asset catalog's source and basis for each appearance, and its
 *  GLB. A GLB still an LFS pointer fails with the command that fetches it. */
export function deriveRosterJets(
  read: (path: string) => Uint8Array,
  list: (dir: string) => string[],
): Record<string, JetGeometry> {
  const text = (path: string) => JSON.parse(new TextDecoder().decode(read(path)));
  const documents = list(ROSTER_DIR)
    .filter((f) => f.endsWith(".json"))
    .sort()
    .map((f) => text(`${ROSTER_DIR}/${f}`));
  const catalog: Record<string, { source?: string; basis_yaw_deg?: number }> =
    text("assets/catalog.json").appearances;
  const out: Record<string, JetGeometry> = {};
  for (const name of fixedWingAppearances(documents)) {
    const entry = catalog[name];
    if (!entry?.source) throw new Error(`assets/catalog.json: ${name} has no source GLB`);
    out[name] = jetGeometry(read(entry.source), entry.source, entry.basis_yaw_deg ?? 0);
  }
  return out;
}
