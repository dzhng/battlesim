// The art rules a unit's model is held to beyond fitting its simulation
// body: tiers that really reduce, so every unit switches detail with zoom;
// dressing outside the hull that can't pass for cover; and budgets per class.
// Like `scenery.ts` for scenery kinds, this table is the art side's, keyed by
// the class a unit draws as: its vehicle class, or soldier. Every number in it is a loose
// tripwire against gross regressions, not a target: when real art exceeds one
// and nothing visibly suffers, raise it (specs/unit-models/choices.md).

import { meshTier, type Scene } from "./scene.ts";
import type { Budget, Finding } from "./schema.ts";

/** The class every soldier appearance draws as. */
export const SOLDIER_CLASS = "soldier";

/** How far a vehicle's dressing (`dressing_*` nodes: antennas, stowage,
 *  crew) may stand outside its hull box, which the rest of the model fits
 *  within the catalog's tolerance. Bulky dressing reaches at most `bulky_m`
 *  past any face, so it never reads as cover the simulation lacks; above
 *  that, only a thin part (an antenna, at most `thin_m` across) may rise,
 *  up to `thin_top_m` over the hull's top. */
export interface DressingAllowance {
  bulky_m: number;
  thin_m: number;
  thin_top_m: number;
}

/** What a class's models are held to: its budget, enforced where set, and
 *  these. */
export interface UnitArtRule extends Budget {
  /** Each tier draws at most this fraction of the triangles of the tier
   *  before it (`structure.tier_ratio`). */
  tier_ratio: number;
  dressing: DressingAllowance;
}

/** The rules by class: a vehicle class (`vehicleClass`) or `SOLDIER_CLASS`.
 *  A class without a row, and a field a row leaves out, take `default`'s.
 *  The budgets are measured on the pilot (slice 14) and empty until then. */
export const UNIT_ART: { default: UnitArtRule } & Record<string, Partial<UnitArtRule>> = {
  default: {
    tier_ratio: 0.9,
    dressing: { bulky_m: 0.3, thin_m: 0.15, thin_top_m: 4 },
  },
};

export const unitArtRule = (cls: string | null): UnitArtRule => ({
  ...UNIT_ART.default,
  ...(cls === null ? {} : UNIT_ART[cls]),
});

/** A unit's (or wreck's) tiers, so it really switches detail with zoom:
 *  every mesh names its tier, rather than being drawn in all of them, and
 *  each tier draws at most `ratio` of the one before. A tier drawing more
 *  than the one before is already `structure.tier_order`. */
export function tierFindings(
  label: string,
  scene: Scene,
  triangles: readonly number[],
  ratio: number,
): Finding[] {
  const out: Finding[] = [];
  const unsuffixed = scene.nodes
    .filter((n) => n.live && n.mesh !== null && meshTier(n.name) === null)
    .map((n) => `"${n.name}"`);
  if (unsuffixed.length)
    out.push({
      code: "structure.tier_unsuffixed",
      severity: "error",
      message: `${label}: mesh ${unsuffixed.slice(0, 5).join(", ")}${unsuffixed.length > 5 ? ` and ${unsuffixed.length - 5} more` : ""} names no tier`,
      fix: "name each mesh <part>_LOD0 (finest) to <part>_LOD3, and leave a part out of the tiers too far away to show it",
    });
  for (let t = 1; t < triangles.length; t++) {
    const [before, now] = [triangles[t - 1], triangles[t]];
    if (now <= before && now > before * ratio)
      out.push({
        code: "structure.tier_ratio",
        severity: "error",
        message: `${label}: tier ${t} draws ${now} triangles, ${(now / before).toFixed(2)} of tier ${t - 1}'s ${before}; each tier draws at most ${ratio} of the one before`,
        fix: "simplify the coarser tier: drop the small parts it is too far to show and lower its segment counts, keeping its silhouette and colour",
      });
  }
  return out;
}
