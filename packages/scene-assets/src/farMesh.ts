// The far mesh (`FAR_TIER`): the optional fifth, coarsest mesh of a static
// prop's state, its source's `_LOD4` meshes alone. The build (`build.ts`)
// gives one only to a static state; the validator (`validate.ts`) holds every
// static state and appearance to these rules.

import { triangleCount } from "./build.ts";
import { FAR_TIER, TIER_COUNT, type Finding, type MeshData } from "./schema.ts";
import type { SceneryRule } from "./scenery.ts";
import { finding } from "./finding.ts";

/** One state's far mesh, where it has one: only a prop kind carries one, it
 *  draws at least one triangle and fewer than tier 3, and a kind that budgets
 *  its tiers budgets it too (its fifth entry, which `budgetFindings` holds). */
export function farFindings(
  label: string,
  kind: string | undefined,
  rule: SceneryRule | undefined,
  tiers: readonly MeshData[],
): Finding[] {
  if (tiers.length <= FAR_TIER) return [];
  if (rule?.footprint.kind !== "prop")
    return [
      finding(
        "structure.tier_count",
        `${label}: names a far mesh (_LOD4), which only a prop kind carries; ${kind ?? "this kind"} is none`,
        `name this art's meshes _LOD0.._LOD${TIER_COUNT - 1}, or unsuffixed`,
      ),
    ];
  const out: Finding[] = [];
  const [far, coarsest] = [triangleCount(tiers[FAR_TIER]), triangleCount(tiers[TIER_COUNT - 1])];
  if (!far)
    out.push(
      finding(
        "structure.tier_count",
        `${label}: its far mesh (_LOD4) has no triangles`,
        "give the _LOD4 meshes geometry, or name none",
      ),
    );
  else if (far >= coarsest)
    out.push(
      finding(
        "structure.tier_order",
        `${label}: its far mesh (_LOD4) draws ${far} triangles, not fewer than tier ${TIER_COUNT - 1}'s ${coarsest}`,
        `simplify the _LOD4 meshes below tier ${TIER_COUNT - 1}: the far mesh is drawn a few pixels tall`,
      ),
    );
  if (rule.tier_triangles && rule.tier_triangles.length <= FAR_TIER)
    out.push(
      finding(
        "budget.tier_triangles",
        `${label}: carries a far mesh, and its kind budgets only its ${TIER_COUNT} tiers`,
        "give the kind's tier_triangles (packages/scene-assets/src/scenery.ts) a fifth entry, the far mesh's",
      ),
    );
  return out;
}

/** Every state of an appearance carries a far mesh, or none does. */
export function farStatesFindings(
  name: string,
  states: readonly { name: string; tiers: readonly MeshData[] }[],
): Finding[] {
  const carrying = states.filter((s) => s.tiers.length > FAR_TIER).map((s) => s.name);
  const bare = states.filter((s) => s.tiers.length <= FAR_TIER).map((s) => s.name);
  if (!carrying.length || !bare.length) return [];
  return [
    finding(
      "structure.tier_count",
      `${name}: ${carrying.join(", ")} carry a far mesh (_LOD4) and ${bare.join(", ")} do not; every state carries one or none does`,
      "name _LOD4 meshes in every state's source, or in none",
    ),
  ];
}
