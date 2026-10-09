// @vitest-environment node
import { expect, test } from "vitest";
import { validatePresentationCapture, type PresentationCapture } from "@web/battle/benchmark/presentationCapture";

const pose = { target: [0, 0] as const, distance: 20, yaw: 0, pitch: 0.2 };
const capture: PresentationCapture = {
  schema: "battle-presentation-capture/v1",
  workload: { id: "menu-reel", fingerprint: "deadbeef", scene: "market-town", map: "menu", encounter: "menu", seed: 1 },
  tickHz: 30, warmTick: 90, side: "blue",
  samples: [{ tick: 90, digest: "0123456789abcdef", publication: [1, 2], camera: pose }],
};

test("accepts a versioned capture with ordered authority samples", () => {
  expect(validatePresentationCapture(capture)).toBe(capture);
});

test("rejects reordered samples and malformed digests", () => {
  expect(() => validatePresentationCapture({ ...capture, samples: [{ ...capture.samples[0], digest: "x" }] })).toThrow(/digest/);
  expect(() => validatePresentationCapture({ ...capture, samples: [capture.samples[0], { ...capture.samples[0], tick: 90 }] })).toThrow(/increase/);
});
