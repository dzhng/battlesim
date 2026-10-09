import { expect, test } from "vitest";
import type { PoseFrame } from "@packages/battle-renderer/src/models/poseDriver";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { UnitCatalog, type CatalogView, type UnitType } from "@packages/scene-assets/src/units";
import {
  inheritWeaponAudio,
  inheritWeaponChoices,
  soundMotion,
} from "@apps/battle-lab/src/soundFeed";

/** A fake catalog: a heavy tracked hull and a supply truck. */
const UNITS = new UnitCatalog({
  units: [
    {
      id: "crusher",
      body: { hull: { weight_class: "heavy" } },
      mobility: { tracked: {} },
      roles: [],
    },
    {
      id: "hauler",
      body: { hull: { weight_class: "medium" } },
      mobility: { wheeled: {} },
      roles: ["logistics"],
    },
  ] as unknown as UnitType[],
} as CatalogView);

// A drawn vehicle as the pose driver hands it to the sound feed.
const vehicle = (unit: number, side: string, kind = "crusher") => ({
  unit,
  side,
  kind,
  position: [0, 0, 0],
  articulation: { travel_l: 0, travel_r: 0, turret_yaw: 0 },
});

test("own and seen enemy vehicles driving backwards both whine; others don't", () => {
  const poses = {
    vehicles: [vehicle(1, "blue"), vehicle(2, "blue"), vehicle(7, "red"), vehicle(8, "red")],
    soldiers: [],
  } as unknown as PoseFrame;
  const motion = soundMotion(poses, UNITS, "blue", new Set([1]), new Set([7]));
  const reverse = Object.fromEntries(motion.vehicles.map((v) => [v.key, v.reverse]));
  // Keys: own unit * 2, enemy unit * 2 + 1.
  expect(reverse).toEqual({ 2: true, 4: false, 15: true, 17: false });
});

test("a drawn vehicle is heard as its class, not its unit type", () => {
  const poses = {
    vehicles: [vehicle(1, "blue", "crusher"), vehicle(2, "blue", "hauler")],
    soldiers: [],
  } as unknown as PoseFrame;
  const motion = soundMotion(poses, UNITS, "blue");
  expect(motion.vehicles.map((v) => v.vehicleClass)).toEqual([
    "tracked_heavy",
    "wheeled_medium_logistics",
  ]);
});

const WEAPONS = {
  missile: {},
  // Derived from a weapon with a motor, two steps away.
  wire_guided: { extends: "missile" },
  heavy_wire_guided: { extends: "wire_guided" },
  rifle: {},
  carbine: { extends: "rifle" },
};

test("a guided missile derived from one with a motor flies with that motor", () => {
  const audio = {
    shots: { default: "shot" },
    impact_scale: { default: 1 },
    blasts: { default: "blast" },
    motors: { missile: { sound: "motor", gain: 0.25 } },
  } as unknown as AudioPresentation;
  const heard = inheritWeaponAudio(audio, WEAPONS);
  expect(heard.motors.heavy_wire_guided).toEqual({ sound: "motor", gain: 0.25 });
  // A rifle round has no motor, nor does anything derived from it.
  expect(heard.motors.carbine).toBeUndefined();
});

test("a derived weapon fires its ancestor's chosen recording, not the global default", () => {
  const rifle = { near: "rifle-near", far: "rifle-far", gain: 1 };
  const catalog = {
    defaults: { default: { near: "plain", far: "plain", gain: 1 }, rifle },
    impacts: {},
  } as unknown as SoundCatalog;
  expect(inheritWeaponChoices(catalog, WEAPONS).defaults.carbine).toEqual(rifle);
});

test("a derived round strikes and glances as its ancestor's, else the contact's default", () => {
  const catalog = {
    defaults: {},
    impacts: {
      hull: { rifle: "rifle-on-steel" },
      ground: { default: "dirt", missile: "dirt-thud" },
    },
  } as unknown as SoundCatalog;
  const { impacts } = inheritWeaponChoices(catalog, WEAPONS);
  expect(impacts.hull.carbine).toBe("rifle-on-steel");
  expect(impacts.ground.heavy_wire_guided).toBe("dirt-thud");
  // No styled ancestor on that contact: its default still decides.
  expect(impacts.hull.heavy_wire_guided).toBeUndefined();
  expect(impacts.ground.carbine).toBeUndefined();
});
