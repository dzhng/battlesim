// Whether a parked aircraft rests on its gear: for every aircraft
// appearance in its rest pose, each node (but the running gear the renderer
// turns, `wheel_*` and `track_*`, and what hangs from it) whose lowest point
// lies within 2 cm of the ground. A rotorcraft on skids or a belly rests on
// the node that carries them; the report names each, to be judged by eye.
import { vec3 } from "math";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { articulatedWorlds } from "@packages/scene-assets/src/pose";
import { gameAppearances } from "./gameAppearances";

/** How near the ground a node's lowest point rests on it, metres. */
const RESTS_M = 0.02;

const isGear = (name: string) => name.startsWith("wheel_") || name.startsWith("track_");

export interface ParkedAircraft {
  kind: string;
  appearance: string;
  /** The model's lowest point at rest, metres above its origin. */
  lowest: number;
  /** Nodes other than gear resting on the ground, with their lowest point. */
  resting: { node: string; lowest: number }[];
}

/** Every aircraft type's hull appearances, loaded in `factions`' looks with
 *  no scenery (as the page loads its own), and what of each rests on the
 *  ground at rest. */
export async function parkedAircraft(
  units: UnitCatalog,
  factions: ReadonlySet<string>,
): Promise<ParkedAircraft[]> {
  const aircraft = units.view.units.filter((t) => "air" in t.mobility && "hull" in t.body);
  const named = aircraft.flatMap((t) =>
    [t.appearance ?? []].flat().map((appearance) => ({ kind: t.id, appearance })),
  );
  const installed = await gameAppearances(
    units,
    named.map((n) => n.appearance),
    units.appearancesFor(aircraft.map((t) => t.id)),
    factions,
    null,
  );
  const out: ParkedAircraft[] = [];
  for (const { kind, appearance } of named) {
    const bundle = installed.appearances.get(appearance)?.bundle;
    if (bundle?.kind !== "articulated") continue;
    const { nodes } = bundle;
    const worlds = articulatedWorlds(nodes);
    const gear = (i: number): boolean => i >= 0 && (isGear(nodes[i].name) || gear(nodes[i].parent));
    const p = vec3.create();
    const lows = nodes.map((node, i) => {
      const mesh = node.tiers[0];
      let low = Infinity;
      for (let v = 0; v < (mesh?.positions.length ?? 0); v += 3)
        low = Math.min(
          low,
          vec3.transformMat4(p, vec3.fromBuffer(p, mesh.positions, v), worlds[i])[2],
        );
      return low;
    });
    const lowest = Math.min(...lows);
    out.push({
      kind,
      appearance,
      lowest,
      resting: nodes
        .map((node, i) => ({ node: node.name, lowest: lows[i], gear: gear(i) }))
        .filter((n) => !n.gear && n.lowest <= lowest + RESTS_M)
        .map(({ node, lowest }) => ({ node, lowest })),
    });
  }
  return out;
}
