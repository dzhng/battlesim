// @vitest-environment node
import { createHash } from "node:crypto";
import { expect, test } from "vitest";
import {
  encodePresentationCapture,
  type PresentationCapture,
  type PresentationFrame,
} from "@web/battle/benchmark/presentationCapture";
import {
  validateVisualPresentation,
  writeVisualPresentation,
  type DrawnPresentation,
  type VisualPresentation,
} from "@web/battle/benchmark/visualPresentation";
import { sunDirection } from "@packages/battle-renderer/src/light/sceneLight";
import { gameLight } from "@apps/battle-lab/src/gameLight";

const frame = (index: number, canvas: [number, number] = [1280, 720]): PresentationFrame => ({
  index,
  reelMs: index * 16,
  tick: 90,
  simTime: 3,
  eye: [0, -40, 6],
  target: [0, 0, 0],
  up: [0, 0.1, 1],
  canvas,
  plate: null,
});

const capture = (frames: PresentationFrame[]): PresentationCapture => ({
  schema: "battle-presentation-capture",
  workload: {
    id: "menu-reel",
    fingerprint: "deadbeef",
    scene: "paris-corner",
    map: "paris-corner",
    encounter: "menu",
    seed: 7,
  },
  tickHz: 30,
  warmTick: 90,
  side: "blue",
  layout: "{}",
  samples: [],
  frames,
});

const drawn: DrawnPresentation = {
  lens: { fovY: 0.8, near: 1, depth: "reverse-z", far: "infinite" },
  light: gameLight,
  layers: ["sky", "terrain", "grass"],
  omissions: [],
};
const mapBytes = new TextEncoder().encode('{"map":"bytes"}');
const sha256 = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");

test("the manifest names its battle by the capture's workload and the hashes of both files", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0), frame(1)]));
  const manifest = JSON.parse(
    await writeVisualPresentation({ captureJson, mapBytes, drawn }),
  ) as VisualPresentation;
  expect(validateVisualPresentation(manifest)).toEqual(manifest);
  expect(manifest).toEqual({
    schema: "native-visual-presentation",
    identity: {
      scene: "paris-corner",
      map: "paris-corner",
      encounter: "menu",
      seed: 7,
      mapHash: sha256(mapBytes),
      captureHash: sha256(captureJson),
    },
    viewport: { width: 1280, height: 720, scale: 1 },
    lens: drawn.lens,
    light: { ...gameLight, sunDirection: Array.from(sunDirection(gameLight)) },
    layers: drawn.layers,
    omissions: [],
  });
});

test("a run whose canvas changed size, or drew nothing, has no one viewport and is refused", async () => {
  const resized = encodePresentationCapture(capture([frame(0), frame(1, [1280, 800])]));
  await expect(writeVisualPresentation({ captureJson: resized, mapBytes, drawn })).rejects.toThrow(
    /canvas/,
  );
  const empty = encodePresentationCapture(capture([]));
  await expect(writeVisualPresentation({ captureJson: empty, mapBytes, drawn })).rejects.toThrow(
    /frame/,
  );
});

test("the validator refuses a versioned id, the hashless identity and an unknown layer", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0)]));
  const manifest = JSON.parse(
    await writeVisualPresentation({ captureJson, mapBytes, drawn }),
  ) as VisualPresentation;
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  expect(reject({ schema: "native-visual-presentation/v1" })).toThrow(/schema/);
  const { mapHash: _, ...hashless } = manifest.identity;
  expect(reject({ identity: hashless })).toThrow(/identity/);
  expect(reject({ layers: ["terrain", "water-boxes"] })).toThrow(/layer/);
  expect(reject({ layers: ["terrain", "terrain"] })).toThrow(/layer/);
  expect(reject({ omissions: [{ owner: "grass", reason: "" }] })).toThrow(/omission/);
});

test("the manifest carries the light the frames were drawn under, refusing one the renderer would refuse", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0)]));
  const manifest = JSON.parse(
    await writeVisualPresentation({ captureJson, mapBytes, drawn }),
  ) as VisualPresentation;
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  const { light: _, ...unlit } = manifest;
  expect(() => validateVisualPresentation(unlit)).toThrow(/malformed/);
  expect(reject({ light: { ...manifest.light, exposure: Number.NaN } })).toThrow(/exposure/);
  // The sun direction is derived from the angles: a stale one is refused.
  expect(reject({ light: { ...manifest.light, sunDirection: [0, 0, 1] } })).toThrow(/sun/);
});
