import type { Vec3 } from "math";
import type { UnitType } from "@packages/scene-assets/src/units.ts";

const armor = { front: 1, side: 1, rear: 1, roof: 1 };
/** A hull type, drawn by appearance `appearance`, with the vehicle class
 *  the catalog's view would give its physics (heavy hull). */
export const hullType = (
  id: string,
  appearance: string,
  half: Vec3,
  mobility: UnitType["mobility"],
  vehicle_class: string | null,
  rest: Partial<UnitType> = {},
): UnitType => ({
  id,
  name: id,
  description: "",
  faction: "test",
  family: "vehicles",
  roles: ["test"],
  cost: 1,
  body: {
    hull: {
      half_extents_m: half,
      eye_m: 2,
      hp: 1,
      armor: { ...armor, ricochet: armor },
      weight_class: "heavy",
      push_class: "heavy",
      wreck: "heavy_wreck",
    },
  },
  mobility,
  concealment: 1,
  vehicle_class,
  sensors: {
    ground_m: 1,
    low_air_m: 1,
    high_air_m: 1,
    sight_shape: { front: 1, side: 1, rear: 1 },
  },
  mounts: [],
  capabilities: {},
  sound: { profile: "vehicle", loudness_m: 1 },
  appearance,
  ...rest,
});
