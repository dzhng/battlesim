import { expect, test } from "vitest";
import type { PoseFrame } from "@packages/battle-renderer/src/models/poseDriver";
import { soundMotion } from "@apps/battle-lab/src/soundFeed";

// A drawn vehicle as the pose driver hands it to the sound feed.
const vehicle = (unit: number, side: string) => ({
  unit,
  side,
  kind: "tank",
  position: [0, 0, 0],
  articulation: { travel_l: 0, travel_r: 0, turret_yaw: 0 },
});

test("own and seen enemy vehicles driving backwards both whine; others don't", () => {
  const poses = {
    vehicles: [vehicle(1, "blue"), vehicle(2, "blue"), vehicle(7, "red"), vehicle(8, "red")],
    soldiers: [],
  } as unknown as PoseFrame;
  const motion = soundMotion(poses, "blue", new Set([1]), new Set([7]));
  const reverse = Object.fromEntries(motion.vehicles.map((v) => [v.key, v.reverse]));
  // Keys: own unit * 2, enemy unit * 2 + 1.
  expect(reverse).toEqual({ 2: true, 4: false, 15: true, 17: false });
});
