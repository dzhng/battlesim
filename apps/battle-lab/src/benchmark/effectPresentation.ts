// What the presentation crate needs to draw a battle's combat effects as
// this page does (the `native-visual-presentation` manifest's `effects`,
// `web/src/battle/benchmark/effectManifest.ts`): the effects' tuning, each
// weapon's rows resolved to its nearest styled ancestor's, each fielded
// type's hull and mount muzzles, the prop kinds that smoke once known, what
// each wreck throws when it cooks off, and how each hull's wreck is fitted
// and stops. Static per battle; the crate runs the per-frame effects
// (`crates/presentation/src/effects`), validating the tuning as it loads it.
import game from "@fixtures/game.json";
import type { Vec3 } from "math";
import { inheritRows } from "@packages/renderer-core/src/kindTable";
import { buildingRemains } from "@packages/scene-assets/src/authority";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { mountMuzzles } from "@packages/scene-assets/src/mountMuzzle";
import type { Bounds } from "@packages/scene-assets/src/schema";
import { SCENERY_KINDS, WRECK_PIECES } from "@packages/scene-assets/src/scenery";
import { topSpeedKmh, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";
import type {
  EffectsManifest,
  WreckLookRecord,
  WreckRecord,
} from "@web/battle/benchmark/effectManifest";

const vec = (v: ArrayLike<number>): Vec3 => [v[0], v[1], v[2]];

/** A round's glow, as review lanes read it (`presentation.effects.tracers`). */
interface TracerRow {
  glow: { color: number[]; intensity: number };
}
const effects = game.presentation.effects as unknown as Record<string, unknown> & {
  tracers: Record<string, TracerRow>;
  flashes: Record<string, unknown>;
  smoke: Record<string, unknown>;
  impact_scale: Record<string, number>;
  blast_scale: Record<string, number>;
};
/** The game's effects, each weapon's round looking as its nearest styled
 *  ancestor's (`inheritRows`) where it has no row of its own. */
export const gameEffects = {
  ...effects,
  tracers: inheritRows(effects.tracers, game.weapons),
  flashes: inheritRows(effects.flashes, game.weapons),
  impact_scale: inheritRows(effects.impact_scale, game.weapons),
  blast_scale: inheritRows(effects.blast_scale, game.weapons),
};

/** The prop kinds that smoke once a side knows of them, each with its look
 *  (a `presentation.effects.smoke` row): what a destroyed building's parts
 *  become, its remains (`ruin`) or its gutted shell (`gutted`), and every
 *  hull's wreck (`wreck`). */
function smokeLooksOf(units: UnitCatalog): Map<string, string> {
  const byKind = new Map<string, string>();
  const { remains, shells } = buildingRemains(units);
  for (const kind of remains) byKind.set(kind, "ruin");
  for (const kind of shells) byKind.set(kind, "gutted");
  for (const id of units.ids) {
    const wreck = units.hull(id)?.wreck;
    if (wreck) byKind.set(wreck, "wreck");
  }
  return byKind;
}

/** What a wreck of `states` throws when it cooks off, in its own frame: its
 *  turret piece's centre, only where it has both pieces (a tank's; others
 *  jolt whole), the piece's underside, and its thrown debris's top. */
export function wreckPieces(states: readonly { name: string; bounds: Bounds }[]): WreckRecord {
  const turret = states.find((s) => s.name === WRECK_PIECES.turret);
  const moving = turret && states.some((s) => s.name === WRECK_PIECES.hull) ? turret : null;
  const debris = states.find((s) => s.name === SCENERY_KINDS.wreck.debris!.state);
  const min = moving?.bounds.min;
  const max = moving?.bounds.max;
  return {
    lies: min && max ? [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2] : null,
    underside: min?.[2] ?? 0,
    debrisTop: debris ? debris.bounds.max[2] : null,
  };
}

/** The `effects` section for a battle fielding `kinds` under `units`, its
 *  wrecks fitted by `fit` from `installed`; a dead hull stops in
 *  `wreckStopS` from its top speed. */
export function describeEffects({
  kinds,
  units,
  fit,
  installed,
  wreckStopS,
}: {
  kinds: readonly string[];
  units: UnitCatalog;
  fit: PropAppearances;
  installed: InstalledAppearances;
  wreckStopS: number;
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
  const wreckLooks: Record<string, WreckLookRecord> = {};
  for (const [unit, { name, footprint }] of Object.entries(fit.describe().wrecks)) {
    const bundle = installed.appearances.get(name)?.bundle;
    const states = bundle?.kind === "static" ? bundle.states : [];
    // A wreck with no whole state makes no transition.
    const whole = states.find((s) => s.name === "default");
    if (!whole) continue;
    wrecks[name] = wreckPieces(states);
    wreckLooks[unit] = {
      appearance: name,
      footprint: vec(footprint),
      bounds: [vec(whole.bounds.min), vec(whole.bounds.max)],
      braking: topSpeedKmh(units.type(unit).mobility) / 3.6 / wreckStopS,
    };
  }
  return {
    presentation: structuredClone(gameEffects) as unknown as Record<string, unknown>,
    types,
    smokes: Object.fromEntries(smokeLooksOf(units)),
    wrecks,
    wreckLooks,
  };
}
