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
  type EffectCount,
  type FrameCount,
  type GrassCount,
  type InstanceCount,
  type ModelCount,
  type VisualPresentation,
} from "@web/battle/benchmark/visualPresentation";

const frame = (
  index: number,
  canvas: [number, number] = [1280, 720],
  plate: string | null = null,
): PresentationFrame => ({
  index,
  reelMs: index * 16,
  tick: 90,
  simTime: 3,
  eye: [0, -40, 6],
  target: [0, 0, 0],
  up: [0, 0.1, 1],
  canvas,
  plate,
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
  layers: ["sky", "terrain", "grass"],
};
const mapBytes = new TextEncoder().encode('{"map":"bytes"}');

const cells = (n: number) => Array.from({ length: 12 }, (_, i) => i * n);
const tiered = (kinds: number, n: number) => Array.from({ length: kinds * 4 }, (_, i) => i * n);
const instanceCounts: InstanceCount[] = [
  {
    plate: "plate-1.png",
    buildings: tiered(2, 3),
    scenery: {
      forest: tiered(2, 2),
      understorey: tiered(2, 0),
      backdrop: tiered(2, 5),
      dressing: tiered(2, 7),
      felled: tiered(2, 0),
      stumps: [1, 0],
    },
    props: [
      { appearance: "abrams_wreck", levels: [0, 1, 0, 2] },
      { appearance: "car_wreck_sedan", levels: [3, 0, 0, 0] },
    ],
  },
];
const grassCounts: GrassCount[] = [{ plate: "plate-1.png", cells: cells(3) }];
const modelCounts: ModelCount[] = [
  { plate: "plate-1.png", soldiers: 9, vehicles: 2, aircraft: 0, corpses: 4, fading: 0 },
];
const effectCounts: EffectCount[] = [
  {
    plate: "plate-1.png",
    running: { tracer: 4, flash: 1, puff: 30 },
    instances: { streak: 8, glow: 2, flipbook: 30, rotor: 0 },
    lights: 3,
  },
];
const platedCapture = () =>
  encodePresentationCapture(capture([frame(0), frame(1, [1280, 720], "plate-1.png")]));
/** The manifest for `captureJson`, drawn with the counts above. */
async function written(
  captureJson: string,
  counts: GrassCount[] = [],
  instances: InstanceCount[] = counts.length ? instanceCounts : [],
  effected: EffectCount[] = counts.length ? effectCounts : [],
) {
  const manifest = await writeVisualPresentation({
    captureJson,
    mapBytes,
    drawn,
    grassCounts: counts,
    instanceCounts: instances,
    modelCounts: counts.length ? modelCounts : [],
    effectCounts: effected,
  });
  return { manifest: JSON.parse(manifest) as VisualPresentation };
}
const sha256 = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");

test("the manifest names its battle by the capture's workload and the hashes of both files", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0), frame(1)]));
  const { manifest } = await written(captureJson);
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
    layers: drawn.layers,
    grassCounts: [],
    instanceCounts: [],
    modelCounts: [],
    effectCounts: [],
    frameCounts: [],
  });
});

test("each plate's model counts ride along, one per plate", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  expect(manifest.modelCounts).toEqual(modelCounts);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  expect(reject({ modelCounts: [{ ...modelCounts[0], corpses: -1 }] })).toThrow(/model counts/);
  await expect(
    writeVisualPresentation({
      captureJson: platedCapture(),
      mapBytes,
      drawn,
      grassCounts,
      instanceCounts,
      modelCounts: [],
      effectCounts,
    }),
  ).rejects.toThrow(/model counts must be one per plate/);
});

test("a run whose canvas changed size, or drew nothing, has no one viewport and is refused", async () => {
  const resized = encodePresentationCapture(capture([frame(0), frame(1, [1280, 800])]));
  await expect(written(resized)).rejects.toThrow(/canvas/);
  const empty = encodePresentationCapture(capture([]));
  await expect(written(empty)).rejects.toThrow(/frame/);
});

test("the validator refuses a versioned id, the hashless identity and an unknown layer", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0)]));
  const { manifest } = await written(captureJson);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  expect(reject({ schema: "native-visual-presentation/v1" })).toThrow(/schema/);
  const { mapHash: _, ...hashless } = manifest.identity;
  expect(reject({ identity: hashless })).toThrow(/identity/);
  expect(reject({ layers: ["terrain", "water-boxes"] })).toThrow(/layer/);
  expect(reject({ layers: ["terrain", "terrain"] })).toThrow(/layer/);
});

test("grass counts hold one row of twelve cells for each plate the capture drew, and only those", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  expect(manifest.grassCounts).toEqual(grassCounts);
  await expect(written(platedCapture(), [])).rejects.toThrow(/plate-1\.png/);
  await expect(
    written(platedCapture(), [...grassCounts, { plate: "plate-0.png", cells: cells(1) }]),
  ).rejects.toThrow(/plate-0\.png/);
  const { grassCounts: _counts, ...countless } = manifest;
  expect(() => validateVisualPresentation(countless)).toThrow(/malformed/);
  const reject = (row: object) => () =>
    validateVisualPresentation({ ...manifest, grassCounts: [row] });
  expect(reject({ plate: "plate-1.png", cells: cells(1).slice(1) })).toThrow(/grass/);
  expect(reject({ plate: "plate-1.png", cells: [-1, ...cells(1).slice(1)] })).toThrow(/grass/);
  expect(reject({ plate: "plate-1.png", cells: [0.5, ...cells(1).slice(1)] })).toThrow(/grass/);
  expect(reject({ plate: "plate-1.png", cells: cells(1), tier: 0 })).toThrow(/grass/);
});

test("instance counts hold one row per plate, a count per module and tier and the scenery's sized by its stumps' appearances", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  expect(manifest.instanceCounts).toEqual(instanceCounts);
  await expect(written(platedCapture(), grassCounts, [])).rejects.toThrow(/instance.*plate-1\.png/);
  const [row] = instanceCounts;
  const reject = (change: object) => () =>
    validateVisualPresentation({ ...manifest, instanceCounts: [{ ...row, ...change }] });
  expect(reject({})).not.toThrow();
  expect(reject({ buildings: [1, 2, 3] })).toThrow(/instance/);
  expect(reject({ buildings: [-1, ...tiered(2, 3).slice(1)] })).toThrow(/instance/);
  expect(reject({ scenery: { ...row.scenery, forest: tiered(3, 1) } })).toThrow(/instance/);
  expect(reject({ scenery: { ...row.scenery, stumps: [1] } })).toThrow(/instance/);
  const { felled: _, ...unfelled } = row.scenery;
  expect(reject({ scenery: unfelled })).toThrow(/instance/);
  expect(reject({ tier: 0 })).toThrow(/instance/);
  const [wreck] = row.props;
  expect(reject({ props: [{ ...wreck, levels: [0, 1, 0, 0, 2] }] })).toThrow(/instance/);
  expect(reject({ props: [wreck, wreck] })).toThrow(/instance/);
  const { props: _props, ...unknowing } = row;
  expect(() => validateVisualPresentation({ ...manifest, instanceCounts: [unknowing] })).toThrow(
    /instance/,
  );
});

test("each plate's effect counts ride along, one per plate", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  expect(manifest.effectCounts).toEqual(effectCounts);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  const [row] = effectCounts;
  expect(reject({ effectCounts: [{ ...row, lights: -1 }] })).toThrow(/effect counts/);
  expect(reject({ effectCounts: [row, row] })).toThrow(/effect counts/);
  await expect(written(platedCapture(), grassCounts, instanceCounts, [])).rejects.toThrow(
    /effect counts must be one per plate/,
  );
});

test("a counted run's frame counts ride along in frame order, each naming a frame the capture drew", async () => {
  const { plate: _i, ...instances } = instanceCounts[0];
  const { plate: _m, ...models } = modelCounts[0];
  const { plate: _e, ...effects } = effectCounts[0];
  const counted = (frame: number): FrameCount => ({ frame, models, effects, drawn: instances });
  const write = (frameCounts: FrameCount[]) =>
    writeVisualPresentation({
      captureJson: platedCapture(),
      mapBytes,
      drawn,
      grassCounts,
      instanceCounts,
      modelCounts,
      effectCounts,
      frameCounts,
    });
  const manifest = JSON.parse(await write([counted(0), counted(1)])) as VisualPresentation;
  expect(manifest.frameCounts).toEqual([counted(0), counted(1)]);
  await expect(write([counted(2)])).rejects.toThrow(/frame counts name frame 2/);
  await expect(write([counted(1), counted(0)])).rejects.toThrow(/frame counts/);
  await expect(write([{ ...counted(0), models: { ...models, fading: -1 } }])).rejects.toThrow(
    /frame counts/,
  );
});
