// What a second client needs to draw a side's props as this page does (the
// `native-visual-presentation` manifest's `props`,
// `web/src/battle/benchmark/propManifest.ts`): the map's props fitted once to
// their appearances (static per map), the tables `PropAppearances.fit` chooses
// a known prop's appearance from, and which map props are a building's parts
// and what a gutted building's parts become. Native fits what the side learns
// per frame (`propAppearance.ts`) and derives fallen buildings by the
// presentation's rule (`static_world/fallen.rs`).
import game from "@fixtures/game.json";
import { buildingRemains } from "@packages/scene-assets/src/authority";
import { STAND_IN_KIT, STAND_IN_MODULE } from "@packages/scene-assets/src/standInKit";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { BuildingIndex } from "@packages/battle-renderer/src/models/buildingReferences";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import type {
  FittedMapProps,
  PropAppearances,
} from "@packages/battle-renderer/src/models/propAppearance";
import {
  NO_PROP,
  PROP_MODEL_FIELDS,
  PROP_MODEL_WORDS,
  type DrawnProps,
  type PropCandidate,
  type PropLook,
} from "@web/battle/benchmark/propManifest";

/** `words` as base64 of their little-endian bytes. */
function base64(words: Uint32Array): string {
  const bytes = new Uint8Array(words.buffer, words.byteOffset, words.byteLength);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000)
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary);
}

/** Per building of `index`, its parts' map prop ids, ascending. */
function partsOf(index: BuildingIndex): number[][] {
  const parts: number[][] = index.parts.map(() => []);
  for (const [prop, building] of index.partBuilding) parts[building].push(prop);
  return parts.map((list) => list.sort((a, b) => a - b));
}

/** The `props` section for a side drawing `world` (the map's props no
 *  knowledge replaces) and `apart` (the map's props drawn from knowledge),
 *  fitting what it learns with `fit`, over `buildings`' parts under `units`. */
export function describeProps({
  world,
  apart,
  fit,
  buildings,
  units,
}: {
  world: readonly ModelInstance[];
  apart: FittedMapProps;
  fit: PropAppearances;
  buildings: BuildingIndex;
  units: UnitCatalog;
}): DrawnProps {
  const looks: PropLook[] = [];
  const lookOf = new Map<string, number>();
  const look = (appearance: string, state: string) => {
    const key = `${appearance}|${state}`;
    if (!lookOf.has(key)) {
      lookOf.set(key, looks.length);
      looks.push({ appearance, state });
    }
    return lookOf.get(key)!;
  };
  const stateOf = (m: ModelInstance) => (m.pose.kind === "static" ? m.pose.state : "default");

  const propOf = Array.from({ length: apart.models.length }, () => NO_PROP);
  for (const [prop, [first, end]] of apart.ranges) {
    if (!Number.isInteger(prop) || prop < 0 || prop >= NO_PROP)
      throw new Error(`map prop ${prop} does not fit a prop model's u32 id`);
    for (let i = first; i < end; i++) propOf[i] = prop;
  }
  const models = [
    ...world.map((m) => ({ m, prop: NO_PROP })),
    ...apart.models.map((m, i) => ({ m, prop: propOf[i] })),
  ];
  const words = new Uint32Array(models.length * PROP_MODEL_WORDS);
  const floats = new Float32Array(words.buffer);
  const at = Object.fromEntries(PROP_MODEL_FIELDS.map((f) => [f.name, f.word]));
  models.forEach(({ m, prop }, i) => {
    const o = i * PROP_MODEL_WORDS;
    floats.set([m.x, m.y, m.z], o + at.position);
    floats[o + at.yaw] = m.yaw;
    floats.set(m.scale ?? [1, 1, 1], o + at.scale);
    words[o + at.look] = look(m.appearance, stateOf(m));
    floats.set(m.tint ?? [1, 1, 1], o + at.tint);
    words[o + at.prop] = prop;
  });

  const described = fit.describe();
  const candidate = ({ name, footprint, paints }: (typeof described.wrecks)[string]) =>
    ({
      look: look(name, "default"),
      footprint: [footprint[0], footprint[1], footprint[2]],
      paints: paints?.map((p) => [p[0], p[1], p[2]]) ?? null,
    }) as PropCandidate;
  const kinds = Object.fromEntries(
    Object.entries(described.kinds).map(([kind, k]) => [
      kind,
      {
        modular: k.modular,
        wreck: k.wreck,
        candidates: k.candidates.map(candidate),
        standIn:
          k.standIn && ([k.standIn[0], k.standIn[1], k.standIn[2]] as [number, number, number]),
      },
    ]),
  );
  const standsIn = Object.values(kinds).some((k) => k.standIn !== null);
  const detail = game.presentation.models;
  return {
    detail: {
      lod_px: [detail.lod_px[0], detail.lod_px[1], detail.lod_px[2]],
      impostor_px: detail.impostor_px,
    },
    looks,
    map: { base64: base64(words), count: models.length },
    fit: {
      kinds,
      wrecks: Object.fromEntries(
        Object.entries(described.wrecks).map(([unit, c]) => [unit, candidate(c)]),
      ),
      standIn: standsIn ? look(STAND_IN_KIT, STAND_IN_MODULE) : null,
    },
    buildings: {
      parts: partsOf(buildings),
      shells: [...buildingRemains(units).shells].sort(),
    },
  };
}
