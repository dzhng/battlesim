// A forest's tree against the simulation: inside the canopy that attenuates
// sight, and built to the one size every species shares.

import type { Authority, Finding, MeshData, Tolerances } from "./schema.ts";
import type { SceneryRule } from "./scenery.ts";

const finding = (code: Finding["code"], message: string, fix: string): Finding => ({
  code,
  severity: "error",
  message,
  fix,
});
const fmt = (n: number) => n.toFixed(3);

/** A tree, unscaled, stands inside the simulation's forest canopy on every
 *  tier, so the drawn crown never rises above or reaches past the foliage
 *  that attenuates sight. */
export function canopyFindings(
  label: string,
  tiers: MeshData[],
  authority: Authority,
  tolerances: Tolerances,
): Finding[] {
  let top = 0;
  let reach = 0;
  for (const { positions: p } of tiers)
    for (let i = 0; i < p.length; i += 3) {
      top = Math.max(top, p[i + 2]);
      reach = Math.max(reach, Math.hypot(p[i], p[i + 1]));
    }
  const out: Finding[] = [];
  if (top > authority.canopy_height_m + tolerances.ground_m)
    out.push(
      finding(
        "fit.canopy",
        `${label}: crown top at ${fmt(top)} m, above the forests' canopy of ${authority.canopy_height_m} m (forests.rule.canopy_height_m)`,
        "lower the crown under the canopy height",
      ),
    );
  if (reach > authority.canopy_radius_m + tolerances.ground_m)
    out.push(
      finding(
        "fit.canopy",
        `${label}: crown reaches ${fmt(reach)} m from the trunk's axis, past the forests' canopy radius of ${authority.canopy_radius_m} m (forests.rule.canopy_radius_m)`,
        "narrow the crown inside the canopy radius",
      ),
    );
  return out;
}

/** The area a mesh's surface encloses where the level plane `z` cuts it:
 *  each triangle the plane crosses gives one edge of the outline, taken
 *  round the way its face looks out, so a closed surface's outline sums to
 *  its area wherever it stands. */
function sectionArea({ positions: p, indices }: MeshData, z: number): number {
  let twice = 0;
  const cut: number[] = [];
  for (let i = 0; i < indices.length; i += 3) {
    const [a, b, c] = [indices[i] * 3, indices[i + 1] * 3, indices[i + 2] * 3];
    cut.length = 0;
    for (const [from, to] of [
      [a, b],
      [b, c],
      [c, a],
    ]) {
      if (p[from + 2] < z === p[to + 2] < z) continue;
      const s = (z - p[from + 2]) / (p[to + 2] - p[from + 2]);
      cut.push(p[from] + s * (p[to] - p[from]), p[from + 1] + s * (p[to + 1] - p[from + 1]));
    }
    if (cut.length !== 4) continue;
    // The face's normal, level: the outline runs with it on its right.
    const nx =
      (p[b + 1] - p[a + 1]) * (p[c + 2] - p[a + 2]) - (p[b + 2] - p[a + 2]) * (p[c + 1] - p[a + 1]);
    const ny = (p[b + 2] - p[a + 2]) * (p[c] - p[a]) - (p[b] - p[a]) * (p[c + 2] - p[a + 2]);
    const along = (cut[2] - cut[0]) * -ny + (cut[3] - cut[1]) * nx;
    twice += Math.sign(along) * (cut[0] * cut[3] - cut[2] * cut[1]);
  }
  return Math.abs(twice) / 2;
}

/** Every tree is one size: its finest tier's top, and its bole's girth at
 *  breast height, stand within the kind's band (`SCENERY_KINDS`), so species
 *  differ in shape alone. */
export function sizeFindings(
  label: string,
  finest: MeshData,
  size: NonNullable<SceneryRule["size"]>,
): Finding[] {
  let top = 0;
  for (let i = 2; i < finest.positions.length; i += 3) top = Math.max(top, finest.positions[i]);
  const bole = Math.sqrt(sectionArea(finest, size.breast_m) / Math.PI);
  const out: Finding[] = [];
  const outside = (value: number, nominal: number) => Math.abs(value / nominal - 1) > size.within;
  const band = (nominal: number) =>
    `${fmt(nominal * (1 - size.within))} to ${fmt(nominal * (1 + size.within))} m`;
  if (outside(top, size.top_m))
    out.push(
      finding(
        "fit.tree_size",
        `${label}: top at ${fmt(top)} m, outside every tree's ${band(size.top_m)}`,
        "build the kind to the one tree height (SCENERY_KINDS.tree.size)",
      ),
    );
  if (outside(bole, size.bole_radius_m))
    out.push(
      finding(
        "fit.tree_size",
        `${label}: the bole is ${fmt(bole)} m in radius at ${size.breast_m} m up, outside every tree's ${band(size.bole_radius_m)}`,
        "build the kind's trunk to the one girth (SCENERY_KINDS.tree.size), clear of its crown at breast height",
      ),
    );
  return out;
}
