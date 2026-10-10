// Where a turning rotor is drawn as a camera's shutter sees it: on the
// rotor as the model poses it, its blades as the model has them, swept by
// the angle it turned since the last drawn frame; only while it flies.
import { expect, test } from "vitest";
import { mat4, vec3, type Vec3 } from "math";
import { DrawnModels, type DrawnRotor } from "@packages/battle-renderer/src/models/drawnModels";
import type { VehiclePose } from "@packages/battle-renderer/src/models/poseDriver";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { REST_ARTICULATION } from "@packages/scene-assets/src/articulation";
import type { ArticulatedBundle } from "@packages/scene-assets/src/schema";
import { validateAppearance } from "@packages/scene-assets/src/validate";
import { HELI_AUTHORITY, HELI_ROTORS, TOLERANCES, heliGlb } from "./sceneAssets/synthetic";

async function heliBundle(): Promise<ArticulatedBundle> {
  const result = await validateAppearance(
    {
      name: "heli",
      entry: { unit: "vehicle", source: "a.glb", basis_yaw_deg: 0 },
      files: { "a.glb": heliGlb() },
    },
    { authority: HELI_AUTHORITY, tolerances: TOLERANCES },
  );
  return result.preview as ArticulatedBundle;
}

const pose = (p: Partial<VehiclePose>): VehiclePose => ({
  unit: 3,
  kind: "heli",
  side: "blue",
  position: [100, 50, 20],
  yaw: Math.PI / 2,
  airborne: true,
  articulation: { ...REST_ARTICULATION, rotor: 0, rotor_blur: 1 },
  tilt: null,
  ...p,
});

/** A drawn heli's rotors over successive frames, each frame's `poses`. */
async function drawnHeli(...frames: Partial<VehiclePose>[]) {
  const bundle = await heliBundle();
  const installed = {
    appearances: new Map([["heli", { bundle, mounts: null }]]),
  } as unknown as InstalledAppearances;
  const drawn = new DrawnModels(
    installed,
    () => ({ appearance: "heli", tint: [1, 1, 1] }),
    HELI_AUTHORITY.units,
  );
  let rotors: DrawnRotor[] = [];
  for (const p of frames.length ? frames : [{}]) {
    drawn.update({ soldiers: [], vehicles: [pose(p)], corpses: [], fading: [] } as never);
    rotors = [];
    drawn.rotors((r) =>
      rotors.push({
        ...r,
        hub: vec3.clone(r.hub),
        axis: vec3.clone(r.axis),
        blade: vec3.clone(r.blade),
        mast: r.mast && { base: vec3.clone(r.mast.base), top: vec3.clone(r.mast.top) },
      }),
    );
  }
  return rotors;
}

const rounded = (v: Vec3) => [...v].map((x) => +x.toFixed(4) + 0);

test("each rotor sits on its drawn hub, across its own axis, with its blades as the model has them", async () => {
  const [main, tail, ...rest] = await drawnHeli();
  expect(rest).toEqual([]);
  // Facing north (+Y): the main rotor's hub, 0.5 m ahead of the middle and
  // 2.4 m up, turns about the vertical, its two blades resting fore and aft.
  expect(rounded(main.hub)).toEqual([100, 50.5, 22.4]);
  expect(rounded(main.axis)).toEqual([0, 0, 1]);
  expect(main.radius).toBeCloseTo(HELI_ROTORS.main, 6);
  expect([main.blades, +main.chord.toFixed(2)]).toEqual([2, 0.3]);
  // Its mast runs from the hub down its axis, as the model has it.
  expect(rounded(main.mast!.base)).toEqual([100, 50.5, 22.2]);
  expect(main.mastRadius).toBeCloseTo(Math.hypot(0.08, 0.08), 4);
  expect(Math.abs(main.blade[1])).toBeCloseTo(1, 4);
  // The tail rotor, on the boom's left (west, facing north), turns across
  // the boom: its axis lies level, east-west.
  expect(tail.radius).toBeCloseTo(HELI_ROTORS.tail, 6);
  expect(rounded(tail.hub)).toEqual([99.1, 46.2, 21.6]);
  expect(Math.abs(tail.axis[0])).toBeCloseTo(1, 6);
  expect(tail.axis[2]).toBeCloseTo(0, 6);
  expect(tail.mast).toBeNull();
});

test("a rotor reports the angle it turned since the last drawn frame: the shutter's sweep", async () => {
  const [first] = await drawnHeli({
    articulation: { ...REST_ARTICULATION, rotor: 4, rotor_blur: 1 },
  });
  // First drawn: nothing turned yet.
  expect(first.turned).toBe(0);
  const [main] = await drawnHeli(
    { articulation: { ...REST_ARTICULATION, rotor: 4, rotor_blur: 1 } },
    { articulation: { ...REST_ARTICULATION, rotor: 6.5, rotor_blur: 1 } },
  );
  // 2.5 m of tip travel on its reach.
  expect(main.turned).toBeCloseTo(2.5 / HELI_ROTORS.main, 6);
  // And its blade has turned with it, about its axis.
  const angle = Math.atan2(main.blade[1], main.blade[0]);
  expect(Math.abs(Math.sin(angle - (Math.PI / 2 + 6.5 / HELI_ROTORS.main)))).toBeLessThan(1e-4);
});

test("a falling airframe's rotor tips with it", async () => {
  const tilt = mat4.fromYRotation(mat4.create(), 0.4);
  const [main] = await drawnHeli({ yaw: 0, tilt });
  // Nose up 0.4 rad about its +Y: the mast leans back, toward -X.
  expect(main.axis[0]).toBeCloseTo(Math.sin(0.4), 6);
  expect(main.axis[2]).toBeCloseTo(Math.cos(0.4), 6);
});

test("nothing on the ground has a rotor to draw", async () => {
  expect(await drawnHeli({ airborne: false })).toEqual([]);
});
