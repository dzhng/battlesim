// @vitest-environment node
import { expect, test } from "vitest";
import { decodePresentationCapture, encodePresentationCapture, validatePresentationCapture, type PresentationCapture } from "@web/battle/benchmark/presentationCapture";

const pose = { target: [0, 0] as const, distance: 20, yaw: 0, pitch: 0.2 };
const capture: PresentationCapture = {
  schema: "battle-presentation-capture/v1",
  workload: { id: "menu-reel", fingerprint: "deadbeef", scene: "market-town", map: "menu", encounter: "menu", seed: 1 },
  tickHz: 30, warmTick: 90, side: "blue", layout: "{\"schema\":\"test\"}",
  samples: [{ tick: 90, digest: "0123456789abcdef", publication: [1, 2], camera: pose }], frames: [{ elapsedMs: 0, tick: 90, camera: pose }],
};

test("accepts a versioned capture with ordered authority samples", () => {
  expect(validatePresentationCapture(capture)).toBe(capture);
});

test("accepts preroll samples before the measurement warm tick", () => {
  expect(validatePresentationCapture({ ...capture, samples: [{ ...capture.samples[0], tick: 1 }, { ...capture.samples[0], tick: 90 }] })).toBeTruthy();
});

test("rejects reordered samples and malformed digests", () => {
  expect(() => validatePresentationCapture({ ...capture, samples: [{ ...capture.samples[0], digest: "x" }] })).toThrow(/digest/);
  expect(() => validatePresentationCapture({ ...capture, samples: [capture.samples[0], { ...capture.samples[0], tick: 90 }] })).toThrow(/increase/);
});

test("rejects publication values that cannot be lossless u32 carriers", () => {
  expect(() => validatePresentationCapture({ ...capture, samples: [{ ...capture.samples[0], publication: [NaN] }] })).toThrow(/u32/);
  expect(() => validatePresentationCapture({ ...capture, samples: [{ ...capture.samples[0], publication: [0x1_0000_0000] }] })).toThrow(/u32/);
});

test("round-trips through the native-tool JSON boundary", () => {
  expect(decodePresentationCapture(encodePresentationCapture(capture))).toEqual(capture);
  expect(() => decodePresentationCapture("{" )).toThrow(/JSON/);
});
