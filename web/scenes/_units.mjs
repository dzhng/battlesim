// The fixtures as the scenes read them: the game's rules and presentation
// (`fixtures/rules.json`, `fixtures/presentation.json`), the street test map, resolved from the saved catalogue, and
// the test catalog (the game's units and the test units labs field), the
// simulation's resolved view, unit and prop types by id. The derived questions mirror
// `packages/scene-assets/src/units.ts`, answered from a type's components.
import { readFile } from "node:fs/promises";
import { loadEncounter, loadMap } from "../src/maps/node.ts";
import { nodeCatalogSet } from "../src/battle/catalog/node.ts";

/** `fixtures/rules.json`. */
export const rules = JSON.parse(
  await readFile(new URL("../../fixtures/rules.json", import.meta.url), "utf8"),
);

/** Visual settings and the audio mix are independent of battle rules. */
export const visuals = JSON.parse(
  await readFile(new URL("../../fixtures/presentation.json", import.meta.url), "utf8"),
);
export const audioMix = JSON.parse(
  await readFile(new URL("../../fixtures/audio.json", import.meta.url), "utf8"),
);

/** The street test map (`fixtures/maps/street`), resolved: the ground of
 *  the battle scenes (`/labs/street`). */
export const streetMap = loadMap("street").definition;
/** The street's battle (`/labs/street`): its saved encounter `attack`. */
export const streetAttack = loadEncounter("street", "attack");

/** The camera curve's pitch at `distance` (the controller's own rule). */
export function curvePitch(distance) {
  const curve = visuals.camera.pitch_curve;
  if (distance <= curve[0][0]) return curve[0][1];
  for (let k = 1; k < curve.length; k++)
    if (distance <= curve[k][0]) {
      const [[d0, p0], [d1, p1]] = [curve[k - 1], curve[k]];
      return p0 + ((p1 - p0) * (distance - d0)) / (d1 - d0);
    }
  return curve.at(-1)[1];
}

const testCatalog = await nodeCatalogSet("test");
const { view } = testCatalog.units;
const byId = new Map(view.units.map((t) => [t.id, t]));

/** The weapon rows as the catalog resolves them (`extends` followed), by
 *  the names mounts give them: what panels read a row's label from. */
export const weapons = testCatalog.weapons;

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

/** Every model a vehicle type draws. */
export const vehicleAppearances = new Set(
  view.units.flatMap((t) => (t.body.hull ? [t.appearance] : [])),
);
