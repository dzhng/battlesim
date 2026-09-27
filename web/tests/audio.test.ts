// @vitest-environment node
import { expect, test } from "vitest";
import { sectorPan } from "@packages/battle-audio/src/soundFrame";
import { describeCue } from "../src/battle/present/captions";

test("captions carry exactly the cue's category, band and direction", () => {
  const text = describeCue(
    { listener: 2, category: "vehicle", sector: 1, band: "far", moving: true },
    "rifle #2",
  );
  expect(text).toBe("Heard engine, moving, far, north-east of rifle #2");
});

test("a cue's stereo pan follows the camera: east is right when looking north", () => {
  const north = [0, 1, 0];
  expect(sectorPan(0, north)).toBeCloseTo(1);
  expect(sectorPan(4, north)).toBeCloseTo(-1);
  expect(sectorPan(2, north)).toBeCloseTo(0);
  // Turn to look east: north is now on the left.
  expect(sectorPan(2, [1, 0, 0])).toBeCloseTo(-1);
});
