// @vitest-environment node
import { expect, test } from "vitest";
import { describeCue, sectorPan } from "../src/battle/present/audio";

test("captions carry exactly the cue's category, band and direction", () => {
  const text = describeCue(
    { listener: 2, category: "vehicle", sector: 1, band: "far", moving: true },
    "rifle #2",
  );
  expect(text).toBe("Heard engine, moving, far, north-east of rifle #2");
});

test("stereo pan follows the camera: east is right when looking north", () => {
  const lookingNorth = -Math.PI / 2; // eye south of the target
  expect(sectorPan(0, lookingNorth)).toBeCloseTo(1);
  expect(sectorPan(4, lookingNorth)).toBeCloseTo(-1);
  expect(sectorPan(2, lookingNorth)).toBeCloseTo(0);
  // Turn to look east: north is now on the left.
  expect(sectorPan(2, Math.PI)).toBeCloseTo(-1);
});
