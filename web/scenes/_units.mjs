// The catalog as the scenes read it: the simulation's resolved view
// (`fixtures/catalog.json`), unit and prop types by id. The derived questions mirror
// `packages/scene-assets/src/units.ts`, answered from a type's components.
import { readFile } from "node:fs/promises";

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
