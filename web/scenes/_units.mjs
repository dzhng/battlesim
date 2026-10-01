// The fixtures as the scenes read them: the game's rules and presentation
// (`fixtures/game.json`), its map, resolved from the saved catalogue, and
// the catalog, the simulation's resolved view
// (`fixtures/catalog.json`), unit and prop types by id. The derived questions mirror
// `packages/scene-assets/src/units.ts`, answered from a type's components.
import { readFile } from "node:fs/promises";
import { loadMap } from "../src/maps/node.ts";

/** `fixtures/game.json`. */
export const game = JSON.parse(
  await readFile(new URL("../../fixtures/game.json", import.meta.url), "utf8"),
);

/** The village's physical map (`fixtures/maps/village`), resolved. */
export const villageMap = loadMap("village").definition;

/** The camera curve's pitch at `distance` (the controller's own rule). */
export function curvePitch(distance) {
  const curve = game.presentation.camera.pitch_curve;
  if (distance <= curve[0][0]) return curve[0][1];
  for (let k = 1; k < curve.length; k++)
    if (distance <= curve[k][0]) {
      const [[d0, p0], [d1, p1]] = [curve[k - 1], curve[k]];
      return p0 + ((p1 - p0) * (distance - d0)) / (d1 - d0);
    }
  return curve.at(-1)[1];
}

const view = JSON.parse(
  await readFile(new URL("../../fixtures/catalog.json", import.meta.url), "utf8"),
);
const byId = new Map(view.units.map((t) => [t.id, t]));

/** The resolved type `id`. */
export const unitType = (id) => {
  const t = byId.get(id);
  if (!t) throw new Error(`no unit type ${id}`);
  return t;
};

/** A vehicle type's hull; null for a squad. */
export const hull = (id) => unitType(id).body.hull ?? null;

/** Whether a published unit (own or identified) is a vehicle. */
export const isVehicle = (u) => hull(u.kind) !== null;

export const hasRole = (id, role) => unitType(id).roles.includes(role);

/** The resolved prop type `id`. */
export const propType = (id) => {
  const t = view.props[id];
  if (!t) throw new Error(`no prop type ${id}`);
  return t;
};

/** A soldier kind's full health. */
export const soldierHp = (kind) => view.soldiers[kind].hp;

/** Every model a vehicle type draws. */
export const vehicleAppearances = new Set(
  view.units.flatMap((t) => (t.body.hull ? [t.appearance] : [])),
);
