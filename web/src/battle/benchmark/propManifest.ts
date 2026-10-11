// The `props` section of the `native-visual-presentation` manifest: what a
// second client needs to draw a side's props as the browser does
// (`packages/battle-renderer/src/models/propAppearance.ts`). The map's props
// are static, so the page fits them and exports the models; what the side
// learns (wrecks, shoved bodies, rubble, fallen buildings) arrives per frame,
// so native ports the fit and reads the tables it chooses from here.
// Plain Node runs the writer, so this imports nothing the renderer owns.
import { finite, hasKeys, isRecord } from "./presentationCapture.ts";

type Vec3 = [number, number, number];

/** A look a prop model draws: an appearance and one of its states (`default`,
 *  or a kit's module for the stand-in box). */
export interface PropLook {
  appearance: string;
  state: string;
}

/** An appearance a prop kind may be fitted to (`PropAppearances` `Candidate`):
 *  its look, its declared half extents and its paints (linear), if any. */
export interface PropCandidate {
  look: number;
  footprint: Vec3;
  paints: Vec3[] | null;
}

/** How a known prop is fitted (`PropAppearances.fit`): per prop kind, the
 *  candidates it chooses among by footprint, whether it repeats one module
 *  along its long side instead of stretching, and the stand-in box's tint
 *  where it has no art; per unit type, its own wreck; and the stand-in box. */
export interface PropFit {
  kinds: Record<
    string,
    { modular: boolean; wreck: boolean; candidates: PropCandidate[]; standIn: Vec3 | null }
  >;
  wrecks: Record<string, PropCandidate>;
  standIn: number | null;
}

/** The manifest's `props`. */
export interface PropsManifest {
  /** `presentation.models`: mesh tiers by projected height, and the card size. */
  detail: { lod_px: [number, number, number]; impostor_px: number };
  looks: PropLook[];
  /** The map's props, fitted: `props-map.u32` (`PROP_MODEL_LAYOUT`). */
  map: PropTableArtifact;
  fit: PropFit;
  /** Per building of `buildings`, its parts' map prop ids; and the prop
   *  types a gutted building's parts become (`buildingRemains` `shells`). */
  buildings: { parts: number[][]; shells: string[] };
}

/** What the page hands over: `PropsManifest` with the map's models inline,
 *  as little-endian words in base64. */
export interface DrawnProps extends Omit<PropsManifest, "map"> {
  map: { base64: string; count: number };
}

export interface PropTableArtifact {
  name: "map";
  sha256: string;
  words: number;
  layout: { kind: "records"; stride: number; count: number; fields: PropTableField[] };
}

interface PropTableField {
  name: string;
  word: number;
  type: "f32" | "u32";
  count: number;
}

/** A fitted map prop model: placed (scaled per axis, turned by `yaw` about
 *  +Z, at `position`), its look, its tint (linear; 1 where it wears none),
 *  and the map prop it draws, `NO_PROP` for one no knowledge replaces. */
export const PROP_MODEL_FIELDS: readonly PropTableField[] = [
  { name: "position", word: 0, type: "f32", count: 3 },
  { name: "yaw", word: 3, type: "f32", count: 1 },
  { name: "scale", word: 4, type: "f32", count: 3 },
  { name: "look", word: 7, type: "u32", count: 1 },
  { name: "tint", word: 8, type: "f32", count: 3 },
  { name: "prop", word: 11, type: "u32", count: 1 },
];
export const PROP_MODEL_WORDS = 12;
export const NO_PROP = 0xffffffff;

/** The file the map's models are written to, beside the manifest. */
export const PROP_TABLE_FILE = "props-map.u32";

/** `drawn`'s map models as an artifact beside the manifest, and the
 *  manifest's `props` naming it. */
export async function propArtifacts(
  drawn: DrawnProps,
  sha256: (bytes: Uint8Array) => Promise<string>,
): Promise<{ props: PropsManifest; artifacts: { file: string; bytes: Uint8Array }[] }> {
  const bytes = new Uint8Array(Buffer.from(drawn.map.base64, "base64"));
  if (bytes.length !== drawn.map.count * PROP_MODEL_WORDS * 4)
    throw new Error(
      `the map's prop models hold ${bytes.length} bytes, not ${drawn.map.count} models`,
    );
  const map: PropTableArtifact = {
    name: "map",
    sha256: await sha256(bytes),
    words: bytes.length / 4,
    layout: {
      kind: "records",
      stride: PROP_MODEL_WORDS,
      count: drawn.map.count,
      fields: [...PROP_MODEL_FIELDS],
    },
  };
  return { props: { ...drawn, map }, artifacts: [{ file: PROP_TABLE_FILE, bytes }] };
}

const text = (value: unknown) => typeof value === "string" && value.length > 0;
const vector = (value: unknown, n: number) =>
  Array.isArray(value) && value.length === n && value.every(finite);
const whole = (value: unknown) => Number.isInteger(value) && (value as number) >= 0;

function isCandidate(value: unknown, looks: number): boolean {
  return (
    hasKeys(value, ["look", "footprint", "paints"]) &&
    whole(value.look) &&
    (value.look as number) < looks &&
    vector(value.footprint, 3) &&
    (value.footprint as number[]).every((h) => h > 0) &&
    (value.paints === null ||
      (Array.isArray(value.paints) &&
        value.paints.length > 0 &&
        value.paints.every((p) => vector(p, 3))))
  );
}

/** `value` as the manifest's `props`, or a refusal naming the part at fault. */
export function validateProps(value: unknown): PropsManifest {
  if (!hasKeys(value, ["detail", "looks", "map", "fit", "buildings"]))
    throw new Error("visual presentation props need detail, looks, map, fit and buildings");
  const { detail, looks, map, fit, buildings } = value;
  if (
    !hasKeys(detail, ["lod_px", "impostor_px"]) ||
    !vector(detail.lod_px, 3) ||
    !finite(detail.impostor_px)
  )
    throw new Error("visual presentation props need the model detail tiers");
  if (
    !Array.isArray(looks) ||
    !looks.every(
      (l) => hasKeys(l, ["appearance", "state"]) && text(l.appearance) && text(l.state),
    ) ||
    new Set(looks.map((l) => `${l.appearance}|${l.state}`)).size !== looks.length
  )
    throw new Error(
      "visual presentation props need distinct looks, each an appearance and a state",
    );
  if (
    !hasKeys(map, ["name", "sha256", "words", "layout"]) ||
    map.name !== "map" ||
    !text(map.sha256) ||
    !hasKeys(map.layout, ["kind", "stride", "count", "fields"]) ||
    map.layout.stride !== PROP_MODEL_WORDS ||
    !whole(map.layout.count) ||
    map.words !== (map.layout.count as number) * PROP_MODEL_WORDS ||
    JSON.stringify(map.layout.fields) !== JSON.stringify(PROP_MODEL_FIELDS)
  )
    throw new Error("visual presentation props need the map's models as a prop model table");
  if (
    !hasKeys(fit, ["kinds", "wrecks", "standIn"]) ||
    !isRecord(fit.kinds) ||
    !isRecord(fit.wrecks)
  )
    throw new Error("visual presentation props need a fit: kinds, wrecks and the stand-in");
  const count = looks.length;
  for (const [kind, entry] of Object.entries(fit.kinds))
    if (
      !hasKeys(entry, ["modular", "wreck", "candidates", "standIn"]) ||
      typeof entry.modular !== "boolean" ||
      typeof entry.wreck !== "boolean" ||
      !Array.isArray(entry.candidates) ||
      !entry.candidates.every((c) => isCandidate(c, count)) ||
      !(entry.standIn === null || vector(entry.standIn, 3)) ||
      (entry.standIn !== null && fit.standIn === null)
    )
      throw new Error(`visual presentation props fit kind ${kind} is malformed`);
  for (const [unit, wreck] of Object.entries(fit.wrecks))
    if (!isCandidate(wreck, count))
      throw new Error(`visual presentation props fit wreck of ${unit} is malformed`);
  if (!(fit.standIn === null || (whole(fit.standIn) && (fit.standIn as number) < count)))
    throw new Error("visual presentation props fit stand-in must be a look");
  if (
    !hasKeys(buildings, ["parts", "shells"]) ||
    !Array.isArray(buildings.parts) ||
    !buildings.parts.every((p) => Array.isArray(p) && p.every(whole)) ||
    new Set(buildings.parts.flat()).size !== buildings.parts.flat().length ||
    !Array.isArray(buildings.shells) ||
    !buildings.shells.every(text)
  )
    throw new Error(
      "visual presentation props need each building's distinct part props and the gutted shells' prop types",
    );
  return value as unknown as PropsManifest;
}
