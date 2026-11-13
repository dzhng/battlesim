// A wreck's parts against its whole: the pieces a cook-off moves apart (its
// hull, its turret) inside the whole they were cut from, and what it threw
// round it as it died within its debris allowance.

import type { AppearanceEntry, Bounds, Finding, MeshData } from "./schema.ts";
import { DEBRIS_NODE, type DebrisAllowance } from "./scenery.ts";
import type { Scene } from "./scene.ts";
import { finding, fmt } from "./finding.ts";

/** The `debris_*` node scene node `i` is or lies under, or null. */
function debrisNode(scene: Scene, i: number): string | null {
  for (let n = i; n >= 0; n = scene.nodes[n].parent)
    if (scene.nodes[n].name.startsWith(DEBRIS_NODE)) return scene.nodes[n].name;
  return null;
}

/** A wreck's debris state (`SceneryRule.debris`): every mesh under a
 *  `debris_*` node, and every point of its finest tier within `reach_m`
 *  across the ground of the box of half extents `half`, and no higher than
 *  `top_m`. */
export function debrisFindings(
  label: string,
  scene: Scene,
  finest: MeshData,
  half: AppearanceEntry["footprint_half_m"],
  allowance: DebrisAllowance,
): Finding[] {
  const out: Finding[] = [];
  const meshes = scene.nodes.filter((n) => n.live && n.mesh !== null);
  const loose = meshes.filter((n) => debrisNode(scene, n.index) === null);
  if (loose.length)
    out.push(
      finding(
        "fit.debris",
        `${label}: mesh ${loose
          .slice(0, 3)
          .map((n) => `"${n.name}"`)
          .join(", ")} lies under no ${DEBRIS_NODE}* node; the debris state carries only debris`,
        `put each thrown piece under a ${DEBRIS_NODE}* node (wreckage.scatter), and leave the wreck's own parts in its other states`,
      ),
    );
  if (!half) return out;
  const p = finest.positions;
  let reach = 0;
  let top = -Infinity;
  for (let k = 0; k < p.length; k += 3) {
    const dx = Math.max(0, Math.abs(p[k]) - half[0]);
    const dy = Math.max(0, Math.abs(p[k + 1]) - half[1]);
    reach = Math.max(reach, Math.hypot(dx, dy));
    top = Math.max(top, p[k + 2]);
  }
  const over = [
    ...(reach > allowance.reach_m
      ? [`${fmt(reach)} m past the box, over its ${allowance.reach_m} m`]
      : []),
    ...(top > allowance.top_m ? [`${fmt(top)} m tall, over its ${allowance.top_m} m`] : []),
  ];
  const names = [...new Set(meshes.map((n) => debrisNode(scene, n.index) ?? n.name))];
  if (over.length)
    out.push(
      finding(
        "fit.debris",
        `${label}: debris (${names.slice(0, 5).join(", ")}) lies ${over.join(" and ")}`,
        "throw the debris nearer and lay it flat; the allowance is SCENERY_KINDS.wreck.debris (packages/scene-assets/src/scenery.ts)",
      ),
    );
  return out;
}

/** Each of a scenery appearance's pieces (`SceneryRule.pieces`) against the
 *  whole it was cut from, its `whole` state: inside it on every face. */
export function pieceFindings(
  label: string,
  states: readonly { name: string; bounds: Bounds }[],
  whole: string,
  pieces: readonly string[],
): Finding[] {
  const of = states.find((s) => s.name === whole)?.bounds;
  if (!of) return [];
  // A millimetre: a piece is cut from the same export as its whole.
  const slack = 0.001;
  return states
    .filter((s) => pieces.includes(s.name))
    .flatMap(({ name, bounds: b }) => {
      const out = [0, 1, 2].flatMap((k) => [
        ...(b.min[k] < of.min[k] - slack ? [`-${"xyz"[k]}`] : []),
        ...(b.max[k] > of.max[k] + slack ? [`+${"xyz"[k]}`] : []),
      ]);
      return out.length
        ? [
            finding(
              "fit.piece",
              `${label} (${name}): reaches past its ${whole} state on ${out.join(", ")}`,
              `export the piece from the same build as ${whole}, where it lies in it`,
            ),
          ]
        : [];
    });
}
