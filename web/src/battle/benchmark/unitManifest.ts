// The `units` section of the `native-visual-presentation` manifest: what a
// second client needs to pose and draw a battle's soldiers, vehicles and
// corpses as the browser does, beside the per-plate counts of what the
// browser's model feed drew. Static per battle, so the page resolves it
// (`apps/battle-lab/src/benchmark/unitPresentation.ts`); what moves per frame
// (interpolation, the pose driver, articulation) native ports as formulas.
// Plain Node runs the writer, so this imports nothing the renderer owns.
import type { Quat, Vec3 } from "math";
import { finite, hasKeys, isRecord } from "./presentationCapture.ts";

/** A node's or joint's local transform (`scene-assets` `Trs`). */
export interface TrsRecord {
  t: Vec3;
  r: Quat;
  s: Vec3;
}

/** A clip at a phase (`PoseRef`). */
export interface PoseRecord {
  clip: string;
  phase: number;
}

/** A unit type as the pose driver and the appearance catalog read it. */
export interface UnitTypeRecord {
  /** Half the distance between its tracks or wheel rows (the pose driver's); null for a squad. */
  hull: { halfTrack: number } | null;
  /** Its vehicle class (`vehicleClass`), the sound row it is heard as; null for a squad. */
  vehicleClass: string | null;
  /** It flies (`airborne`). */
  airborne: boolean;
  /** How its model draws each mount, in mount order (`mountRoles`). */
  roles: string[];
  /** Per mount: its first weapon (the round a launch is matched by), the mount
   *  it rides (`on`, as an index; null: the hull), and the hold an operator
   *  takes with it active (`operator_appearance.active_pose`). */
  mounts: { weapon: string; on: number | null; activePose: PoseRecord | null }[];
  /** Per side, the installed appearances `AppearanceCatalog.resolve` picks
   *  from, each in the side's faction's look: a hull's model, each slot's
   *  soldier set, and each mount's operator's sets (null: no operator kit). */
  draws: Record<"blue" | "red", UnitDraws>;
}

export interface UnitDraws {
  hull: string[];
  slots: string[][];
  operated: ({ active: string[]; carried: string[] } | null)[];
}

/** A clip of a dense clip table (`clipTable.ts` `TableClip`): its first float. */
export interface ClipRecord {
  offset: number;
  frames: number;
  duration: number;
  rate: number;
  loop: boolean;
  stride: number | null;
}

/** An articulated bundle's rig (`articulationRig`), its node indices into `nodes`. */
export interface RigRecord {
  turret: number;
  gun: number;
  hmg: number;
  hmgGun: number;
  pitch: { gun: [number, number]; hmg: [number, number] };
  /** How far the gun runs back, metres; null: unbounded. */
  stroke: number | null;
  wheels: { node: number; radius: number; left: boolean }[];
  tracks: { node: number; left: boolean; linkPitch: number }[];
  /** Each rotor, as the effects draw its blur (`RotorShape`). */
  rotors: {
    node: number;
    radius: number;
    blades: number;
    chord: number;
    rest: number;
    mast_below: number;
    mast_above: number;
    mast_radius: number;
  }[];
  deploy: { node: number; start: number; end: number; move: Vec3; turn: Vec3 }[];
  /** Each articulated mount's muzzle node (`MOUNT_NODES`), by role, where
   *  the model has it: a flash sits on it. */
  muzzles: Record<string, number>;
}

/** A body's weapon `muzzle` socket: the joint it hangs from and its offset
 *  in that joint's space; null where it has none. */
export type AppearanceMuzzle = { joint: number; offset: TrsRecord } | null;

export type AppearanceRecord =
  | {
      kind: "skinned";
      /** In the skeleton clips' joint order, parents first. */
      joints: { name: string; parent: number; bind: TrsRecord }[];
      /** The clip table (`tables`) its clips are sampled from. */
      table: string;
      clips: Record<string, ClipRecord>;
      farPose: PoseRecord;
      /** Each way the body lies dead (`INFANTRY_CORPSES`), in that order. */
      corpsePoses: PoseRecord[];
      /** Its weapon's `muzzle` socket: the joint it hangs from and its
       *  offset; null where it has none. A flash sits on it. */
      muzzle: AppearanceMuzzle;
      /** Each joint's inverse bind (column-major): a palette joint is its
       *  world times this. */
      inverseBinds: number[][];
      tiers: number;
      /** How big it is drawn (`boundsSize`), for its tier: the far pose,
       *  then each way it lies dead. */
      bounds: { size: number; radius: number }[];
    }
  | {
      kind: "articulated";
      nodes: { name: string; parent: number; bind: TrsRecord }[];
      rig: RigRecord;
      tiers: number;
      /** How big it is drawn at rest (`boundsSize`), for its tier. */
      bounds: { size: number; radius: number }[];
    };

/** One dense clip table (`buildClipTable`): `SAMPLE_FLOATS` f32 a joint sample. */
export interface ClipTableArtifact {
  name: string;
  sha256: string;
  words: number;
  layout: { kind: "records"; stride: number; count: number; fields: TableField[] };
}

interface TableField {
  name: string;
  word: number;
  type: "f32";
  count: number;
}

/** The manifest's `units`. */
export interface UnitsManifest {
  /** `presentation.pose`, as the pose driver read it. */
  pose: Record<string, unknown>;
  /** The rules' `cover.lean_hold_s`: how long a soldier holds his firing pose. */
  leanHold: number;
  /** `presentation.models`: mesh tiers by projected height, and the card size. */
  detail: { lod_px: [number, number, number]; impostor_px: number };
  /** Each side's tint (linear rgb) on tint-masked surfaces (the catalog's `sides`). */
  tints: Record<"blue" | "red", Vec3>;
  types: Record<string, UnitTypeRecord>;
  appearances: Record<string, AppearanceRecord>;
  tables: ClipTableArtifact[];
}

/** What the page hands over: `UnitsManifest` with each clip table's floats
 *  inline, as little-endian bytes in base64 (they cross from the page as JSON). */
export interface DrawnUnits extends Omit<UnitsManifest, "tables"> {
  tables: { name: string; base64: string }[];
}

/** What a plate's frame posed, by kind (`modelCounts`): living and dying
 *  soldiers, ground vehicles, aircraft, lying corpses and corpses sinking
 *  away past the cap. */
export interface ModelCount {
  plate: string;
  soldiers: number;
  vehicles: number;
  aircraft: number;
  corpses: number;
  fading: number;
}

export const MODEL_COUNT_KINDS = ["soldiers", "vehicles", "aircraft", "corpses", "fading"] as const;

/** Floats per joint sample (`clipTable.ts` `SAMPLE_FLOATS`): rotation, translation, scale. */
const SAMPLE_FLOATS = 8;

/** The file a clip table is written to, beside the manifest. */
export const unitTableFile = (name: string) => `units-${name}.u32`;

/** The layout of a clip table of `floats` floats. */
function clipLayout(floats: number): ClipTableArtifact["layout"] {
  return {
    kind: "records",
    stride: SAMPLE_FLOATS,
    count: floats / SAMPLE_FLOATS,
    fields: [
      { name: "rotation", word: 0, type: "f32", count: 4 },
      { name: "translation", word: 4, type: "f32", count: 3 },
      { name: "scale", word: 7, type: "f32", count: 1 },
    ],
  };
}

/** `drawn`'s clip tables as artifacts beside the manifest, and the manifest's
 *  `units` naming them. */
export async function unitArtifacts(
  drawn: DrawnUnits,
  sha256: (bytes: Uint8Array) => Promise<string>,
): Promise<{ units: UnitsManifest; artifacts: { file: string; bytes: Uint8Array }[] }> {
  const artifacts: { file: string; bytes: Uint8Array }[] = [];
  const tables: ClipTableArtifact[] = [];
  for (const { name, base64 } of drawn.tables) {
    const bytes = new Uint8Array(Buffer.from(base64, "base64"));
    if (bytes.length % (SAMPLE_FLOATS * 4) !== 0)
      throw new Error(`clip table ${name} holds ${bytes.length} bytes, not whole joint samples`);
    artifacts.push({ file: unitTableFile(name), bytes });
    tables.push({
      name,
      sha256: await sha256(bytes),
      words: bytes.length / 4,
      layout: clipLayout(bytes.length / 4),
    });
  }
  return { units: { ...drawn, tables }, artifacts };
}

const text = (value: unknown) => typeof value === "string" && value.length > 0;
const index = (value: unknown) => Number.isInteger(value) && (value as number) >= -1;
const vector = (value: unknown, n: number) =>
  Array.isArray(value) && value.length === n && value.every(finite);
const isTrs = (value: unknown) =>
  hasKeys(value, ["t", "r", "s"]) && vector(value.t, 3) && vector(value.r, 4) && vector(value.s, 3);
const isPose = (value: unknown) =>
  hasKeys(value, ["clip", "phase"]) && text(value.clip) && finite(value.phase);
const names = (value: unknown) => Array.isArray(value) && value.every(text);

/** Whether `nodes` is a parents-first hierarchy of named, bound nodes. */
const isHierarchy = (nodes: unknown) =>
  Array.isArray(nodes) &&
  nodes.length > 0 &&
  nodes.every(
    (n, i) =>
      hasKeys(n, ["name", "parent", "bind"]) &&
      text(n.name) &&
      Number.isInteger(n.parent) &&
      (n.parent as number) >= -1 &&
      (n.parent as number) < i &&
      isTrs(n.bind),
  );

function isRig(rig: unknown, nodes: number): boolean {
  if (
    !hasKeys(rig, [
      "turret",
      "gun",
      "hmg",
      "hmgGun",
      "pitch",
      "stroke",
      "wheels",
      "tracks",
      "rotors",
      "deploy",
      "muzzles",
    ])
  )
    return false;
  const node = (v: unknown) => index(v) && (v as number) < nodes;
  const pitch = rig.pitch;
  return (
    [rig.turret, rig.gun, rig.hmg, rig.hmgGun].every(node) &&
    hasKeys(pitch, ["gun", "hmg"]) &&
    vector(pitch.gun, 2) &&
    vector(pitch.hmg, 2) &&
    (rig.stroke === null || (finite(rig.stroke) && rig.stroke >= 0)) &&
    Array.isArray(rig.wheels) &&
    rig.wheels.every(
      (w) =>
        hasKeys(w, ["node", "radius", "left"]) &&
        node(w.node) &&
        finite(w.radius) &&
        w.radius > 0 &&
        typeof w.left === "boolean",
    ) &&
    Array.isArray(rig.tracks) &&
    rig.tracks.every(
      (t) =>
        hasKeys(t, ["node", "left", "linkPitch"]) &&
        node(t.node) &&
        typeof t.left === "boolean" &&
        finite(t.linkPitch) &&
        t.linkPitch > 0,
    ) &&
    Array.isArray(rig.rotors) &&
    rig.rotors.every(
      (r) =>
        hasKeys(r, [
          "node",
          "radius",
          "blades",
          "chord",
          "rest",
          "mast_below",
          "mast_above",
          "mast_radius",
        ]) &&
        node(r.node) &&
        finite(r.radius) &&
        r.radius > 0 &&
        Number.isInteger(r.blades) &&
        (r.blades as number) >= 0 &&
        [r.chord, r.rest, r.mast_below, r.mast_above, r.mast_radius].every(finite),
    ) &&
    Array.isArray(rig.deploy) &&
    rig.deploy.every(
      (m) =>
        hasKeys(m, ["node", "start", "end", "move", "turn"]) &&
        node(m.node) &&
        finite(m.start) &&
        finite(m.end) &&
        (m.end as number) > (m.start as number) &&
        vector(m.move, 3) &&
        vector(m.turn, 3),
    ) &&
    isRecord(rig.muzzles) &&
    Object.values(rig.muzzles).every((n) => node(n) && (n as number) >= 0)
  );
}

function isClip(value: unknown): boolean {
  return (
    hasKeys(value, ["offset", "frames", "duration", "rate", "loop", "stride"]) &&
    Number.isInteger(value.offset) &&
    (value.offset as number) >= 0 &&
    Number.isInteger(value.frames) &&
    (value.frames as number) > 0 &&
    finite(value.duration) &&
    value.duration > 0 &&
    finite(value.rate) &&
    value.rate > 0 &&
    typeof value.loop === "boolean" &&
    (value.stride === null || (finite(value.stride) && value.stride > 0))
  );
}

/** A tier count and `count` sizes (`boundsSize`). */
const isDrawn = (value: Record<string, unknown>, count: number) =>
  Number.isInteger(value.tiers) &&
  (value.tiers as number) >= 0 &&
  Array.isArray(value.bounds) &&
  value.bounds.length === count &&
  value.bounds.every((b) => hasKeys(b, ["size", "radius"]) && finite(b.size) && finite(b.radius));

function isAppearance(value: unknown, tables: ReadonlySet<string>): boolean {
  if (!isRecord(value)) return false;
  if (value.kind === "skinned")
    return (
      hasKeys(value, [
        "kind",
        "joints",
        "table",
        "clips",
        "farPose",
        "corpsePoses",
        "muzzle",
        "inverseBinds",
        "tiers",
        "bounds",
      ]) &&
      isHierarchy(value.joints) &&
      Array.isArray(value.inverseBinds) &&
      value.inverseBinds.length === (value.joints as unknown[]).length &&
      value.inverseBinds.every((m) => Array.isArray(m) && m.length === 16 && m.every(finite)) &&
      isDrawn(value, 1 + ((value.corpsePoses as unknown[] | undefined) ?? []).length) &&
      tables.has(value.table as string) &&
      isRecord(value.clips) &&
      Object.values(value.clips).every(isClip) &&
      isPose(value.farPose) &&
      Array.isArray(value.corpsePoses) &&
      value.corpsePoses.every(isPose) &&
      (value.muzzle === null ||
        (hasKeys(value.muzzle, ["joint", "offset"]) &&
          Number.isInteger(value.muzzle.joint) &&
          (value.muzzle.joint as number) >= 0 &&
          (value.muzzle.joint as number) < (value.joints as unknown[]).length &&
          isTrs(value.muzzle.offset)))
    );
  return (
    value.kind === "articulated" &&
    hasKeys(value, ["kind", "nodes", "rig", "tiers", "bounds"]) &&
    isHierarchy(value.nodes) &&
    isDrawn(value, 1) &&
    isRig(value.rig, (value.nodes as unknown[]).length)
  );
}

function isDraws(value: unknown, appearances: Record<string, unknown>): boolean {
  const known = (list: unknown) =>
    names(list) && (list as string[]).every((n) => Object.hasOwn(appearances, n));
  return (
    hasKeys(value, ["hull", "slots", "operated"]) &&
    known(value.hull) &&
    Array.isArray(value.slots) &&
    value.slots.every(known) &&
    Array.isArray(value.operated) &&
    value.operated.every(
      (o) =>
        o === null || (hasKeys(o, ["active", "carried"]) && known(o.active) && known(o.carried)),
    )
  );
}

function isType(value: unknown, appearances: Record<string, unknown>): boolean {
  if (!hasKeys(value, ["hull", "vehicleClass", "airborne", "roles", "mounts", "draws"]))
    return false;
  const { hull, roles, mounts, draws } = value;
  return (
    (hull === null || (hasKeys(hull, ["halfTrack"]) && finite(hull.halfTrack))) &&
    (value.vehicleClass === null || text(value.vehicleClass)) &&
    typeof value.airborne === "boolean" &&
    names(roles) &&
    Array.isArray(mounts) &&
    mounts.length === (roles as unknown[]).length &&
    mounts.every(
      (m) =>
        hasKeys(m, ["weapon", "on", "activePose"]) &&
        text(m.weapon) &&
        (m.on === null || (Number.isInteger(m.on) && (m.on as number) >= 0)) &&
        (m.activePose === null || isPose(m.activePose)),
    ) &&
    hasKeys(draws, ["blue", "red"]) &&
    isDraws(draws.blue, appearances) &&
    isDraws(draws.red, appearances)
  );
}

/** Refuses a `units` section a reader could not follow, naming the part at fault. */
export function validateUnits(value: unknown): UnitsManifest {
  if (!hasKeys(value, ["pose", "leanHold", "detail", "tints", "types", "appearances", "tables"]))
    throw new Error("visual presentation units are malformed");
  if (!isRecord(value.pose)) throw new Error("visual presentation units need the pose feel");
  if (!finite(value.leanHold) || value.leanHold < 0)
    throw new Error("visual presentation units need a lean hold of seconds");
  const detail = value.detail;
  if (
    !hasKeys(detail, ["lod_px", "impostor_px"]) ||
    !vector(detail.lod_px, 3) ||
    !finite(detail.impostor_px)
  )
    throw new Error("visual presentation units need the model detail tiers");
  if (
    !hasKeys(value.tints, ["blue", "red"]) ||
    !vector(value.tints.blue, 3) ||
    !vector(value.tints.red, 3)
  )
    throw new Error("visual presentation units need each side's tint");
  const tables = value.tables;
  if (
    !Array.isArray(tables) ||
    !tables.every(
      (t) =>
        hasKeys(t, ["name", "sha256", "words", "layout"]) &&
        text(t.name) &&
        typeof t.sha256 === "string" &&
        /^[0-9a-f]{64}$/.test(t.sha256) &&
        Number.isInteger(t.words) &&
        (t.words as number) % SAMPLE_FLOATS === 0 &&
        JSON.stringify(t.layout) === JSON.stringify(clipLayout(t.words as number)),
    )
  )
    throw new Error("visual presentation units' clip tables must be hashed f32 joint samples");
  const tableNames = new Set((tables as { name: string }[]).map((t) => t.name));
  const appearances = value.appearances;
  if (
    !isRecord(appearances) ||
    !Object.values(appearances).every((a) => isAppearance(a, tableNames))
  )
    throw new Error(
      "visual presentation units' appearances must be skinned bodies on a listed clip table or rigged vehicles",
    );
  if (!isRecord(value.types) || !Object.values(value.types).every((t) => isType(t, appearances)))
    throw new Error(
      "visual presentation units' types need a hull, mounts and per-side appearance sets the appearances list",
    );
  return value as unknown as UnitsManifest;
}

/** Whether `value` is one plate's model counts. */
export const isModelCount = (value: unknown): boolean =>
  hasKeys(value, ["plate", ...MODEL_COUNT_KINDS]) &&
  text(value.plate) &&
  MODEL_COUNT_KINDS.every((k) => Number.isInteger(value[k]) && (value[k] as number) >= 0);
