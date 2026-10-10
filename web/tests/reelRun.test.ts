// @vitest-environment node
import { expect, test } from "vitest";
import { createReelRun, grassCells } from "@apps/battle-lab/src/benchmark/reelRun";
import type { UploadedTables } from "@packages/battle-renderer/src/frame/wordTables";
import type { BackdropScene } from "@apps/battle-lab/src/menuReel";

const pose = { target: [0, 0] as [number, number], distance: 40, yaw: 0, pitch: 0.2 };
const scene: BackdropScene = {
  map: "fake-menu",
  encounter: "e",
  seed: 3,
  warm_s: 2,
  reel: { fade_s: 0.5, shots: [{ seconds: 2, from: pose, to: { ...pose, distance: 20 } }] },
};
const camera = {
  ...pose,
  target: [0, 0, 0] as [number, number, number],
  fovY: 1,
  aspect: 1,
  near: 0.1,
};
const blank = () => Promise.resolve(new Blob());
const noProbe = () => {
  throw new Error("a probe ran outside plate mode");
};
/** A drawn frame at `now`, sim time `simTime`. */
const drawn = (
  now: number,
  simTime = 2,
  readback: () => Promise<Blob> = blank,
  grassClumps: () => Promise<{ root: [number, number, number] }[]> = noProbe,
) => ({
  now,
  cpuMs: 1,
  camera,
  simTime,
  canvas: [1280, 720] as const,
  readback,
  grassClumps,
});
/** A level camera 40 m east of (5, 6, 1), looking west: screen right is +y, up is +z. */
const level = { ...camera, target: [5, 6, 1] as [number, number, number], yaw: 0, pitch: 0 };

test("a reel records only drawn intervals after warm-up, finishes once, and uses wall time for the camera", () => {
  const results: unknown[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r));
  run.pilot.frame!(drawn(100));
  expect(run.recording.frames).toEqual([]);
  run.scripted.onWarm();
  run.pilot.frame!(drawn(1000));
  expect(run.pilot.pose(2000)?.distance).toBe(30);
  run.scripted.onTick({ tick: 90, stepMs: 2, bytes: 100 });
  run.pilot.frame!(drawn(2000));
  run.pilot.frame!(drawn(3000));
  run.pilot.frame!(drawn(3100));
  expect(run.recording.frames.map((f) => f.intervalMs)).toEqual([1000, 1000]);
  expect(results).toHaveLength(1);
  expect(results[0]).toMatchObject({ outcome: { status: "complete" }, startTick: 60, endTick: 90 });
});

test("a reel capture copies packed publications before their credits are released", () => {
  const results: unknown[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r));
  run.scripted.onWarm();
  run.pilot.frame!(drawn(1000));
  run.scripted.onPublication!({
    tick: 61,
    digest: "0123456789abcdef",
    packed: [1, 2, 3],
    warm: true,
  });
  expect(run.captureSamples()).toEqual([
    expect.objectContaining({ tick: 61, digest: "0123456789abcdef", publication: [1, 2, 3] }),
  ]);
});

test("cancelling during preparation cannot later publish a complete result", () => {
  const results: unknown[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r));
  run.cancel();
  run.scripted.onWarm();
  run.pilot.frame!(drawn(1000));
  run.pilot.frame!(drawn(4000));
  run.fail("late failure");
  expect(results).toHaveLength(1);
  expect(results[0]).toMatchObject({ outcome: { status: "cancelled" } });
  expect(run.recording.frames).toEqual([]);
});

test("each drawn frame records the view it drew in sim space, and a plate from each shot's middle", async () => {
  type Result = { frames: unknown[]; plates: { file: string; png: Promise<Blob> }[] };
  const results: Result[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r as unknown as Result), {
    plates: true,
  });
  run.scripted.onWarm();
  run.pilot.frame!(drawn(1000));
  run.scripted.onTick({ tick: 61, stepMs: 2, bytes: 100 });
  const reads: number[] = [];
  const read = (now: number) => () => {
    reads.push(now);
    return Promise.resolve(new Blob([String(now)]));
  };
  // Frames 0.5 s and 1.1 s into the 2 s shot: the second is the first past its middle.
  run.pilot.frame!({ ...drawn(1500, 2.0, read(1500)), camera: level });
  run.pilot.frame!({ ...drawn(2100, 2.6, read(2100), () => Promise.resolve([])), camera: level });
  run.pilot.frame!(drawn(3000, 3.5, read(3000)));
  expect(reads).toEqual([2100]);
  const [result] = results;
  // As written to disk, where a signed zero is a zero.
  expect(JSON.parse(JSON.stringify(result.frames.slice(0, 2)))).toEqual([
    {
      index: 0,
      reelMs: 500,
      tick: 61,
      simTime: 2,
      eye: [45, 6, 1],
      target: [5, 6, 1],
      up: [0, 0, 1],
      canvas: [1280, 720],
      plate: null,
    },
    expect.objectContaining({ index: 1, reelMs: 1100, simTime: 2.6, plate: "plate-1.png" }),
  ]);
  expect(result.plates.map((p) => p.file)).toEqual(["plate-1.png"]);
  expect(await (await result.plates[0].png).text()).toBe("2100");
});

test("grass is counted by the screen cell its roots project into, row-major from the top-left", () => {
  const cells = grassCells(
    [
      [5, 7, 1.5], // just right of and above the middle: second row, third column
      [5, 7, 1.5],
      [5, -24, 19], // up and to the left
      [5, 36, -17], // down and to the right
      [60, 6, 1], // behind the eye
      [5, 200, 1], // off the side of the screen
    ],
    level,
    [1280, 720],
  );
  expect(cells).toEqual([1, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1]);
});

test("a plate frame counts the grass its own frame grew and keeps the tables it drew with", async () => {
  type Result = {
    plates: { file: string; grass: Promise<number[]> }[];
    tables: UploadedTables | null;
  };
  const results: Result[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r as unknown as Result), {
    plates: true,
  });
  const tables = { terrain: {}, grass: {} } as unknown as UploadedTables;
  let exported = 0;
  run.pilot.attach!({
    adapter: "test",
    light: {} as never,
    stats: () => ({}) as never,
    tables: () => (exported++, tables),
  });
  run.scripted.onWarm();
  run.pilot.frame!(drawn(1000));
  // Only the plate frame (1.1 s into the 2 s shot) reads the grass back.
  const grown = () => Promise.resolve([{ root: [5, 7, 1.5] as [number, number, number] }]);
  run.pilot.frame!({ ...drawn(1500), camera: level });
  run.pilot.frame!({ ...drawn(2100, 2.6, blank, grown), camera: level });
  run.pilot.frame!(drawn(3000));
  const [result] = results;
  expect(await result.plates[0].grass).toEqual([0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
  expect(result.tables).toBe(tables);
  expect(exported).toBe(1);
});

test("without plates the run reads nothing back", () => {
  const results: { tables: UploadedTables | null }[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r));
  run.pilot.attach!({
    adapter: "test",
    light: {} as never,
    stats: () => ({}) as never,
    tables: noProbe,
  });
  run.scripted.onWarm();
  for (const now of [1000, 1500, 2100, 3000]) run.pilot.frame!({ ...drawn(now), camera: level });
  expect(results[0].tables).toBeNull();
});
