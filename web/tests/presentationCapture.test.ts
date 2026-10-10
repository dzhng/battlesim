// @vitest-environment node
import { expect, test } from "vitest";
import {
  decodePresentationCapture,
  encodePresentationCapture,
  plateFile,
  validatePresentationCapture,
  type PresentationCapture,
  type PresentationFrame,
} from "@web/battle/benchmark/presentationCapture";

const frame = (index: number, at: Partial<PresentationFrame> = {}): PresentationFrame => ({
  index,
  reelMs: 16 * index,
  tick: 90 + index,
  simTime: 3 + index / 30,
  eye: [10, -40, 6],
  target: [10, 0, 1],
  up: [0, 0.15, 0.99],
  canvas: [1280, 720],
  plate: null,
  ...at,
});

const capture: PresentationCapture = {
  schema: "battle-presentation-capture",
  workload: {
    id: "menu-reel",
    fingerprint: "deadbeef",
    scene: "market-town",
    map: "menu",
    encounter: "menu",
    seed: 1,
  },
  tickHz: 30,
  warmTick: 90,
  side: "blue",
  layout: '{"schema":"test"}',
  samples: [{ tick: 90, digest: "0123456789abcdef", publication: [1, 2] }],
  frames: [frame(0), frame(1, { plate: plateFile(1) }), frame(2)],
};

const reject = (change: Partial<PresentationCapture>) => () =>
  validatePresentationCapture({ ...capture, ...change });

test("accepts a capture whose frames record the drawn view and its plate", () => {
  expect(validatePresentationCapture(capture)).toBe(capture);
});

test("rejects the versioned schema id and the old pose-only frame", () => {
  expect(
    reject({ schema: "battle-presentation-capture/v1" as "battle-presentation-capture" }),
  ).toThrow(/schema/);
  const pose = { target: [0, 0], distance: 20, yaw: 0, pitch: 0.2 };
  expect(
    reject({ frames: [{ elapsedMs: 0, tick: 90, camera: pose } as unknown as PresentationFrame] }),
  ).toThrow(/frame/);
  expect(
    reject({
      samples: [{ ...capture.samples[0], camera: pose } as PresentationCapture["samples"][number]],
    }),
  ).toThrow(/sample/);
});

test("frame records run in order: index by position, and reel time, tick and sim time never go back", () => {
  expect(reject({ frames: [frame(0), frame(2)] })).toThrow(/index/);
  expect(reject({ frames: [frame(0), frame(1, { reelMs: -1 })] })).toThrow(/reelMs/);
  expect(reject({ frames: [frame(0), frame(1, { tick: 80 })] })).toThrow(/tick/);
  expect(reject({ frames: [frame(0), frame(1, { simTime: 2 })] })).toThrow(/simTime/);
});

test("a frame's view is three finite sim-space points and a whole-pixel canvas", () => {
  expect(
    reject({ frames: [frame(0, { eye: [0, 0] as unknown as [number, number, number] })] }),
  ).toThrow(/eye/);
  expect(reject({ frames: [frame(0, { up: [0, 0, NaN] })] })).toThrow(/up/);
  expect(reject({ frames: [frame(0, { canvas: [1280.5, 720] })] })).toThrow(/canvas/);
});

test("a plate is named by its own frame's index", () => {
  expect(reject({ frames: [frame(0, { plate: plateFile(3) })] })).toThrow(/plate/);
});

test("rejects reordered samples, malformed digests and lossy publication words", () => {
  expect(reject({ samples: [{ ...capture.samples[0], digest: "x" }] })).toThrow(/digest/);
  expect(reject({ samples: [capture.samples[0], capture.samples[0]] })).toThrow(/increase/);
  expect(reject({ samples: [{ ...capture.samples[0], publication: [0x1_0000_0000] }] })).toThrow(
    /u32/,
  );
});

test("round-trips through the native-tool JSON boundary", () => {
  expect(decodePresentationCapture(encodePresentationCapture(capture))).toEqual(capture);
  expect(() => decodePresentationCapture("{")).toThrow(/JSON/);
});
