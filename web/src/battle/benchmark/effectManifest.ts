// The `effects` section of the `native-visual-presentation` manifest: what a
// second client needs to turn a side's publications into the browser's combat
// effects (`packages/battle-renderer/src/effects/effectFrame.ts` over
// `apps/battle-lab/src/effectFeed.ts`): the effects' tuning as the browser
// read it, and the catalog rows the effects read (hulls and mount muzzles,
// which prop kinds smoke, and what a cook-off's wreck throws). Static per
// battle, so the page resolves it (`benchmark/effectPresentation.ts`); the
// effects themselves are per frame, so native ports them as formulas. Beside
// it, `effectCounts` holds what each plate's frame drew of them.
// Plain Node runs the writer, so this imports nothing the renderer owns.
import type { Vec3 } from "math";
import { finite, hasKeys, isRecord } from "./presentationCapture.ts";

/** A hull mount's muzzle model (`mountMuzzle.ts` `MountMuzzle`): the mount it
 *  rides (`on`, an index; null: the hull), its pivot and its carrier's ring
 *  in the hull's frame at rest, and its muzzle from the pivot. */
export interface MuzzleRecord {
  on: number | null;
  pivot: Vec3;
  ring: Vec3;
  muzzle: Vec3;
}

/** A thrown wreck's pieces, as a cook-off reads them (`cookOffs.ts`
 *  `transitionOf`), in the wreck's own frame: its turret piece's centre
 *  (null: it throws none), how high the piece's underside lies, and how tall
 *  its thrown debris lies (null: it throws none). */
export interface WreckRecord {
  lies: Vec3 | null;
  underside: number;
  debrisTop: number | null;
}

/** The manifest's `effects`. */
export interface EffectsManifest {
  /** `presentation.effects` as the effects read it: each weapon's rows
   *  resolved to its nearest styled ancestor's (`gameEffects`). */
  presentation: Record<string, unknown>;
  /** Per unit type: its hull's half extents (null: infantry, whose rifles
   *  raise no dust), and each mount's muzzle model (null: a hand weapon). */
  types: Record<string, { half: Vec3 | null; muzzles: (MuzzleRecord | null)[] }>;
  /** The prop kinds that smoke once the side knows them, each with its look
   *  (a `presentation.smoke` row): ruins, gutted shells and wrecks. */
  smokes: Record<string, string>;
  /** Per wreck appearance, the pieces a cook-off throws. */
  wrecks: Record<string, WreckRecord>;
}

/** What a plate's frame drew of the effects (`EffectFrame.counts`): those
 *  running at its clock by kind, its instances by shape, and its lights. */
export interface EffectCount {
  plate: string;
  running: Record<string, number>;
  instances: Record<string, number>;
  lights: number;
}

const vector = (value: unknown) =>
  Array.isArray(value) && value.length === 3 && value.every(finite);
const whole = (value: unknown) => Number.isInteger(value) && (value as number) >= 0;
const counts = (value: unknown) => isRecord(value) && Object.values(value).every(whole);

const isMuzzle = (value: unknown) =>
  value === null ||
  (hasKeys(value, ["on", "pivot", "ring", "muzzle"]) &&
    (value.on === null || whole(value.on)) &&
    vector(value.pivot) &&
    vector(value.ring) &&
    vector(value.muzzle));

const isWreck = (value: unknown) =>
  hasKeys(value, ["lies", "underside", "debrisTop"]) &&
  (value.lies === null || vector(value.lies)) &&
  finite(value.underside) &&
  (value.debrisTop === null || finite(value.debrisTop));

/** Refuses an `effects` section a reader could not follow, naming the part at fault. */
export function validateEffects(value: unknown): EffectsManifest {
  if (!hasKeys(value, ["presentation", "types", "smokes", "wrecks"]))
    throw new Error("visual presentation effects are malformed");
  const looks = isRecord(value.presentation) ? value.presentation.smoke : null;
  if (!isRecord(looks))
    throw new Error("visual presentation effects need presentation.effects with its smoke looks");
  if (
    !isRecord(value.types) ||
    !Object.values(value.types).every(
      (t) =>
        hasKeys(t, ["half", "muzzles"]) &&
        (t.half === null || vector(t.half)) &&
        Array.isArray(t.muzzles) &&
        t.muzzles.every(isMuzzle),
    )
  )
    throw new Error("visual presentation effects' types need a hull's half extents and muzzles");
  if (
    !isRecord(value.smokes) ||
    !Object.values(value.smokes).every(
      (look) => typeof look === "string" && Object.hasOwn(looks, look),
    )
  )
    throw new Error(
      "visual presentation effects' smokes must name presentation.effects.smoke looks",
    );
  if (!isRecord(value.wrecks) || !Object.values(value.wrecks).every(isWreck))
    throw new Error("visual presentation effects' wrecks need their pieces");
  return value as unknown as EffectsManifest;
}

/** Whether `value` is one plate's effect counts. */
export const isEffectCount = (value: unknown): boolean =>
  hasKeys(value, ["plate", "running", "instances", "lights"]) &&
  typeof value.plate === "string" &&
  value.plate.length > 0 &&
  counts(value.running) &&
  counts(value.instances) &&
  whole(value.lights);
