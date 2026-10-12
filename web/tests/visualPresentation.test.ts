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
  type GrassCount,
  type InstanceCount,
  type VisualPresentation,
} from "@web/battle/benchmark/visualPresentation";
import { sunDirection } from "@packages/battle-renderer/src/light/sceneLight";
import { gameLight } from "@apps/battle-lab/src/gameLight";
import type { DrawnUnits, ModelCount } from "@web/battle/benchmark/unitManifest";
import { NO_PROP, PROP_MODEL_WORDS, type DrawnProps } from "@web/battle/benchmark/propManifest";
import type { EffectCount, EffectsManifest } from "@web/battle/benchmark/effectManifest";

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

/** Two joint samples of one clip, as a page's clip table holds them. */
const CLIP_FLOATS = Float32Array.from([0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0.6, 0.8, 0.2, 0, 0, 1]);
const REST = { t: [0, 0, 0], r: [0, 0, 0, 1], s: [1, 1, 1] } as const;
const IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
/** A squad of one soldier on a one-clip table and a tank on a two-node rig. */
function units(): DrawnUnits {
  const soldier = (look: string) => ({ hull: [], slots: [[look]], operated: [null] });
  return {
    pose: { gait: { walk_mps: 0.25 } },
    leanHold: 1.5,
    detail: { lod_px: [150, 60, 24], impostor_px: 10 },
    tints: { blue: [1, 1, 1], red: [1.18, 1, 0.78] },
    types: {
      rifle: {
        hull: null,
        airborne: false,
        roles: ["hand"],
        mounts: [{ weapon: "rifle", on: null, activePose: null }],
        draws: { blue: soldier("rifleman"), red: soldier("rifleman") },
      },
      tank: {
        hull: { halfTrack: 1.4 },
        airborne: false,
        roles: ["gun"],
        mounts: [{ weapon: "cannon", on: null, activePose: null }],
        draws: {
          blue: { hull: ["tank"], slots: [], operated: [null] },
          red: { hull: ["tank"], slots: [], operated: [null] },
        },
      },
    },
    appearances: {
      rifleman: {
        kind: "skinned",
        joints: [
          { name: "root", parent: -1, bind: structuredClone(REST) as never },
          { name: "spine", parent: 0, bind: structuredClone(REST) as never },
        ],
        table: "clips-0",
        clips: { idle: { offset: 0, frames: 1, duration: 1, rate: 30, loop: true, stride: null } },
        farPose: { clip: "idle", phase: 0 },
        corpsePoses: [{ clip: "idle", phase: 1 }],
        muzzle: { joint: 1, offset: structuredClone(REST) as never },
        inverseBinds: [IDENTITY, IDENTITY],
        tiers: 1,
        bounds: [
          { size: 1.8, radius: 1 },
          { size: 1.8, radius: 1.9 },
        ],
      },
      tank: {
        kind: "articulated",
        nodes: [
          { name: "hull", parent: -1, bind: structuredClone(REST) as never },
          { name: "turret", parent: 0, bind: structuredClone(REST) as never },
        ],
        rig: {
          turret: 1,
          gun: -1,
          hmg: -1,
          hmgGun: -1,
          pitch: { gun: [-0.17, 0.35], hmg: [-0.17, 0.79] },
          stroke: null,
          wheels: [],
          tracks: [],
          rotors: [],
          deploy: [],
          muzzles: { gun: 1 },
        },
        tiers: 1,
        bounds: [{ size: 3.6, radius: 4 }],
      },
    },
    tables: [{ name: "clips-0", base64: Buffer.from(CLIP_FLOATS.buffer).toString("base64") }],
  };
}

/** Two fitted map models (a lamp no knowledge replaces, a car map prop 7),
 *  each building's parts, and a fit whose parked car paints from one look
 *  and whose tank wrecks as another. */
const MAP_WORDS = (() => {
  const words = new Uint32Array(2 * PROP_MODEL_WORDS);
  const floats = new Float32Array(words.buffer);
  floats.set([10, 20, 0, 0.5, 1, 1, 1], 0);
  words[7] = 0;
  floats.set([1, 1, 1], 8);
  words[11] = NO_PROP;
  floats.set([30, 40, 0, 1.5, 1.1, 0.9, 1], PROP_MODEL_WORDS);
  words[PROP_MODEL_WORDS + 7] = 1;
  floats.set([0.3, 0.3, 0.31], PROP_MODEL_WORDS + 8);
  words[PROP_MODEL_WORDS + 11] = 7;
  return words;
})();
const props = (): DrawnProps => ({
  detail: { lod_px: [150, 60, 24], impostor_px: 12 },
  looks: [
    { appearance: "lamp", state: "default" },
    { appearance: "parked_car", state: "default" },
    { appearance: "abrams_wreck", state: "default" },
  ],
  map: { base64: Buffer.from(MAP_WORDS.buffer).toString("base64"), count: 2 },
  fit: {
    kinds: {
      parked_car: {
        modular: false,
        wreck: false,
        candidates: [{ look: 1, footprint: [2.2, 0.9, 0.72], paints: [[0.3, 0.3, 0.31]] }],
        standIn: null,
      },
      wreck: { modular: false, wreck: true, candidates: [], standIn: null },
    },
    wrecks: { abrams: { look: 2, footprint: [4, 1.8, 1.2], paints: null } },
    standIn: null,
  },
  buildings: { parts: [[0, 1], [2]], shells: ["gutted_shell"] },
});

/** The tank's hull and gun muzzle, a wreck that smokes, and its wreck's pieces. */
const effects = (): EffectsManifest => ({
  presentation: { capacity: 16, smoke: { wreck: { burn_s: 60 } } },
  types: {
    rifle: { half: null, muzzles: [null] },
    tank: {
      half: [3.2, 1.8, 1.2],
      muzzles: [{ on: null, pivot: [0, 0, 2], ring: [0, 0, 0], muzzle: [5, 0, 0] }],
    },
  },
  smokes: { heavy_wreck: "wreck" },
  wrecks: { abrams_wreck: { lies: [0.1, 0, 2.1], underside: 1.6, debrisTop: 0.5 } },
  wreckLooks: {
    tank: {
      appearance: "abrams_wreck",
      footprint: [4, 1.8, 1.2],
      bounds: [
        [-4, -1.8, 0],
        [4, 1.8, 2.4],
      ],
      braking: 4,
    },
  },
});

const drawn: DrawnPresentation = {
  lens: { fovY: 0.8, near: 1, depth: "reverse-z", far: "infinite" },
  light: {
    ...gameLight,
    sunDirection: Array.from(sunDirection(gameLight)) as [number, number, number],
  },
  layers: ["sky", "terrain", "grass"],
  omissions: [],
  units: units(),
  props: props(),
  effects: effects(),
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
    knownProps: [
      { appearance: "abrams_wreck", levels: [0, 1, 0, 0, 2] },
      { appearance: "car_wreck_sedan", levels: [3, 0, 0, 0, 0] },
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
/** The manifest and its artifacts for `captureJson`, drawn with the counts above. */
async function written(
  captureJson: string,
  counts: GrassCount[] = [],
  instances: InstanceCount[] = counts.length ? instanceCounts : [],
  effected: EffectCount[] = counts.length ? effectCounts : [],
) {
  const { manifest, artifacts } = await writeVisualPresentation({
    captureJson,
    mapBytes,
    drawn,
    grassCounts: counts,
    instanceCounts: instances,
    modelCounts: counts.length ? modelCounts : [],
    effectCounts: effected,
  });
  return { manifest: JSON.parse(manifest) as VisualPresentation, artifacts };
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
    light: drawn.light,
    layers: drawn.layers,
    omissions: [],
    grassCounts: [],
    instanceCounts: [],
    units: expect.any(Object),
    modelCounts: [],
    props: expect.any(Object),
    effects: drawn.effects,
    effectCounts: [],
  });
});

test("the units' clip tables are artifacts after the props', and each plate's model counts ride along", async () => {
  const { manifest, artifacts } = await written(platedCapture(), grassCounts);
  const bytes = new Uint8Array(CLIP_FLOATS.buffer);
  const { tables: clipTables, ...described } = manifest.units;
  const { tables: _, ...page } = drawn.units;
  expect(described).toEqual(page);
  expect(clipTables).toEqual([
    {
      name: "clips-0",
      sha256: sha256(bytes),
      words: CLIP_FLOATS.length,
      layout: {
        kind: "records",
        stride: 8,
        count: 2,
        fields: [
          { name: "rotation", word: 0, type: "f32", count: 4 },
          { name: "translation", word: 4, type: "f32", count: 3 },
          { name: "scale", word: 7, type: "f32", count: 1 },
        ],
      },
    },
  ]);
  expect(artifacts.at(-1)).toEqual({ file: "units-clips-0.u32", bytes });
  expect(manifest.modelCounts).toEqual(modelCounts);

  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  const [rifleman] = Object.values(manifest.units.appearances);
  expect(reject({ units: { ...manifest.units, tables: [] } })).toThrow(/units/);
  expect(
    reject({
      units: {
        ...manifest.units,
        appearances: { ...manifest.units.appearances, rifleman: { ...rifleman, joints: [] } },
      },
    }),
  ).toThrow(/units/);
  const { rifle, ...rest } = manifest.units.types;
  const stray = {
    ...rifle,
    draws: { ...rifle.draws, red: { hull: [], slots: [["ghost"]], operated: [null] } },
  };
  expect(reject({ units: { ...manifest.units, types: { ...rest, rifle: stray } } })).toThrow(
    /types/,
  );
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

test("the map's fitted props are an artifact before the units', the only other one, and the fit and the parts ride along", async () => {
  const { manifest, artifacts } = await written(platedCapture(), grassCounts);
  const bytes = new Uint8Array(MAP_WORDS.buffer);
  const { map, ...described } = manifest.props;
  const { map: _, ...page } = drawn.props;
  expect(described).toEqual(page);
  expect(map).toEqual({
    name: "map",
    sha256: sha256(bytes),
    words: 2 * PROP_MODEL_WORDS,
    layout: expect.objectContaining({ kind: "records", stride: PROP_MODEL_WORDS, count: 2 }),
  });
  expect(artifacts.at(-2)).toEqual({ file: "props-map.u32", bytes });
  expect(artifacts).toHaveLength(1 + manifest.units.tables.length);

  const reject = (change: object) => () =>
    validateVisualPresentation({ ...manifest, props: { ...manifest.props, ...change } });
  const { buildings: parts, fit } = manifest.props;
  expect(reject({ buildings: { ...parts, parts: [[0, 1], [1]] } })).toThrow(/part props/);
  expect(reject({ buildings: { parts: parts.parts } })).toThrow(/props/);
  const car = fit.kinds.parked_car;
  const astray = { ...car, candidates: [{ ...car.candidates[0], look: 3 }] };
  expect(reject({ fit: { ...fit, kinds: { ...fit.kinds, parked_car: astray } } })).toThrow(
    /parked_car/,
  );
  expect(reject({ map: { ...map, words: 12 } })).toThrow(/map/);
  await expect(
    writeVisualPresentation({
      captureJson: platedCapture(),
      mapBytes,
      drawn: { ...drawn, props: { ...drawn.props, map: { ...drawn.props.map, count: 3 } } },
      grassCounts,
      instanceCounts,
      modelCounts,
      effectCounts,
    }),
  ).rejects.toThrow(/3 models/);
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
  expect(reject({ omissions: [{ owner: "grass", reason: "" }] })).toThrow(/omission/);
});

test("the manifest carries the light the frames were drawn under, with a unit sun direction", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0)]));
  const { manifest } = await written(captureJson);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  const { light: _, ...unlit } = manifest;
  expect(() => validateVisualPresentation(unlit)).toThrow(/malformed/);
  expect(reject({ light: { ...manifest.light, sunDirection: [0, 0, 2] } })).toThrow(/sun/);
  const { sunDirection: _sun, ...sunless } = manifest.light;
  expect(reject({ light: sunless })).toThrow(/sun/);
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
  const [wreck] = row.knownProps;
  expect(reject({ knownProps: [{ ...wreck, levels: [0, 1, 0, 0] }] })).toThrow(/instance/);
  expect(reject({ knownProps: [wreck, wreck] })).toThrow(/instance/);
  const { knownProps: _known, ...unknowing } = row;
  expect(() => validateVisualPresentation({ ...manifest, instanceCounts: [unknowing] })).toThrow(
    /instance/,
  );
});

test("the effects ride along as the page described them, and each plate's effect counts beside them", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  expect(manifest.effects).toEqual(drawn.effects);
  expect(manifest.effectCounts).toEqual(effectCounts);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  const { effects: section } = manifest;
  expect(reject({ effects: { ...section, smokes: { heavy_wreck: "inferno" } } })).toThrow(
    /smoke looks/,
  );
  const tank = { ...section.types.tank, half: [3.2, 1.8] };
  expect(reject({ effects: { ...section, types: { ...section.types, tank } } })).toThrow(/types/);
  const { wrecks: _, ...wreckless } = section;
  expect(reject({ effects: wreckless })).toThrow(/effects/);
  const [row] = effectCounts;
  expect(reject({ effectCounts: [{ ...row, lights: -1 }] })).toThrow(/effect counts/);
  expect(reject({ effectCounts: [row, row] })).toThrow(/effect counts/);
  await expect(written(platedCapture(), grassCounts, instanceCounts, [])).rejects.toThrow(
    /effect counts must be one per plate/,
  );
});
