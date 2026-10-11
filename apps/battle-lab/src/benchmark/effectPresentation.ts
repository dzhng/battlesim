// What a second client needs to draw a battle's combat effects as this page
// does (the `native-visual-presentation` manifest's `effects`,
// `web/src/battle/benchmark/effectManifest.ts`): the effects' tuning as
// `effectFeed.ts` reads it, each fielded type's hull and mount muzzles, the
// prop kinds that smoke once known, and what each unit's wreck throws when it
// cooks off. Static per battle; native ports the per-frame effects.
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { mountMuzzles } from "@packages/scene-assets/src/mountMuzzle";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";
import type { EffectsManifest, WreckRecord } from "@web/battle/benchmark/effectManifest";
import { gameEffects, smokeLooksOf } from "../effectFeed";
import { wreckPieces } from "../cookOffs";

type Vec3 = [number, number, number];
const vec = (v: ArrayLike<number>): Vec3 => [v[0], v[1], v[2]];

/** The `effects` section for a battle fielding `kinds` under `units`, its
 *  wrecks fitted by `fit` from `installed`. */
export function describeEffects({
  kinds,
  units,
  fit,
  installed,
}: {
  kinds: readonly string[];
  units: UnitCatalog;
  fit: PropAppearances;
  installed: InstalledAppearances;
}): EffectsManifest {
  const types: EffectsManifest["types"] = {};
  for (const kind of [...new Set(kinds)].sort()) {
    const type = units.type(kind);
    const half = units.hull(kind)?.half_extents_m;
    types[kind] = {
      half: half ? vec(half) : null,
      muzzles: mountMuzzles(type.mounts).map(
        (m) => m && { on: m.on, pivot: vec(m.pivot), ring: vec(m.ring), muzzle: vec(m.muzzle) },
      ),
    };
  }
  const wrecks: Record<string, WreckRecord> = {};
  for (const { name } of Object.values(fit.describe().wrecks)) {
    const bundle = installed.appearances.get(name)?.bundle;
    const states = bundle?.kind === "static" ? bundle.states : [];
    // A wreck with no whole state makes no transition (`transitionOf`).
    if (states.some((s) => s.name === "default")) wrecks[name] = wreckPieces(states);
  }
  return {
    presentation: structuredClone(gameEffects) as unknown as Record<string, unknown>,
    types,
    smokes: Object.fromEntries(smokeLooksOf(units)),
    wrecks,
  };
}
