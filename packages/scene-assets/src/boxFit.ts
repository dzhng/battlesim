// Art against the simulation's box: a vehicle's hull, and a scenery prop's
// footprint, face by face.

import type { Vec3 } from "math";
import { positionsBounds } from "./pose.ts";
import { SCENERY_KINDS } from "./scenery.ts";
import type { AppearanceEntry, Finding, MeshData, Tolerances } from "./schema.ts";
import { finding, fmt } from "./finding.ts";

export interface ExtentRule {
  code: "fit.hull_extents" | "fit.footprint";
  /** What the box is, for the message: `units.tank.body.hull.half_extents_m [..]`. */
  rule: string;
  tolerance: number;
  fix: string;
}

/** The five faces (±x, ±y, top) of `positions` against a box of half extents
 *  `half` standing on the ground, in both directions. */
export function extentFindings(
  label: string,
  positions: Float32Array,
  half: Vec3,
  { code, rule, tolerance, fix }: ExtentRule,
): Finding[] {
  const b = positionsBounds(positions);
  const faces: [string, number, number][] = [
    ["-x", b.min[0], -half[0]],
    ["+x", b.max[0], half[0]],
    ["-y", b.min[1], -half[1]],
    ["+y", b.max[1], half[1]],
    ["+z", b.max[2], 2 * half[2]],
  ];
  const off = faces.filter(([, model, box]) => Math.abs(model - box) > tolerance);
  return off.length
    ? [
        finding(
          code,
          `${label}: ${off.map(([face, model, box]) => `${face} face at ${fmt(model)} m vs ${fmt(box)} m`).join("; ")} (${rule}, tolerance ${tolerance})`,
          fix,
        ),
      ]
    : [];
}

/** A scenery appearance that stands for a simulation prop, against its box. */
export function footprintFindings(
  entry: AppearanceEntry,
  states: { name: string; tiers: MeshData[] }[],
  tolerances: Tolerances,
  label: string,
): Finding[] {
  const footprint = SCENERY_KINDS[entry.scenery ?? ""]?.footprint;
  if (footprint?.kind !== "prop") return [];
  const tolerance = Math.max(tolerances.footprint_m, footprint.rubble_m ?? 0);
  const half = entry.footprint_half_m;
  if (!half)
    return [
      finding(
        "fit.footprint",
        `${label}: no footprint_half_m; a ${entry.scenery} prop is fitted to the simulation's box`,
        "set footprint_half_m in the catalog entry to the half extents of the box the simulation places (a map placement, or a wreck's hull)",
      ),
    ];
  return states.flatMap((state) =>
    extentFindings(`${label} (${state.name})`, state.tiers[0].positions, half, {
      code: "fit.footprint",
      rule: `footprint_half_m [${half.join(", ")}]`,
      tolerance,
      fix: "fit the art to the simulation's box, or widen footprint_m for this appearance in the catalog (overhangs, rubble)",
    }),
  );
}
