import { vec3 } from "math";
import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import type { BenchmarkOutcome } from "@web/battle/benchmark/report";
import {
  createGpuMat4,
  createProjectedPoint,
  eyePosition,
  projectPoint,
  screenAxes,
  viewProjMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
import { liveCamera } from "@packages/renderer-core/src/viewportCamera";
import { plateShot, sampleReel, type CaptureScene } from "../capture/capture";
import type { ScriptedSim } from "../useSimSession";
import type { FramePlate, ViewportPilot } from "../viewportRender";
import {
  plateFile,
  type PresentationFrame,
  type PresentationSample,
} from "@web/battle/benchmark/presentationCapture";
import { GRASS_CELLS, type FrameCount, type Lens } from "@web/battle/benchmark/visualPresentation";

/** A control plate: the frame it names read back (`FramePlate`) with what it
 *  drew of the buildings and scenery, and the grass clumps it grew counted
 *  per screen cell (`grassCells`). */
export interface ReelPlate extends Omit<FramePlate, "clumps"> {
  file: string;
  grass: Promise<number[]>;
}

const _cellProjection = createGpuMat4();
const _cellPoint = createProjectedPoint();
const _cellRoot = vec3.create();

/** How many of `roots` fall in each cell of the `GRASS_CELLS` screen grid
 *  as `camera` draws a `canvas`-sized frame, row-major from the top-left;
 *  a root behind the eye or off the screen is in none. */
export function grassCells(
  roots: readonly (readonly [number, number, number])[],
  camera: Camera3DParams,
  [width, height]: readonly [number, number],
): number[] {
  const [cols, rows] = GRASS_CELLS;
  const cells = Array.from({ length: cols * rows }, () => 0);
  viewProjMatrix(_cellProjection, liveCamera({ camera3d: camera, width, height }));
  for (const root of roots) {
    const { ndc, clipW } = projectPoint(
      _cellPoint,
      _cellProjection,
      vec3.set(_cellRoot, root[0], root[1], root[2]),
    );
    const x = (ndc[0] + 1) / 2;
    const y = (1 - ndc[1]) / 2;
    if (clipW <= 0 || x < 0 || x >= 1 || y < 0 || y >= 1) continue;
    cells[Math.floor(y * rows) * cols + Math.floor(x * cols)]++;
  }
  return cells;
}

export interface ReelResult {
  scene: CaptureScene;
  outcome: BenchmarkOutcome;
  recording: BenchmarkRecording;
  startTick: number;
  endTick: number;
  /** The rate the page's presentation ran at (the authority's `ready`). */
  tickHz: number;
  layout: string;
  adapter: string | null;
  capture: readonly PresentationSample[];
  frames: readonly PresentationFrame[];
  /** The lens the frames drew through; null before the first frame. */
  lens: Lens | null;
  plates: readonly ReelPlate[];
  /** What the counted frames drew (`counts`), in frame order. */
  counts: readonly FrameCount[];
}

/** Of the frames between tick boundaries, a counting run counts one in this many. */
const COUNT_BETWEEN = 4;

/** One complete scene; preparation and inter-scene loading have no frame
 *  samples. With `plates`, each shot's middle frame is read back as a
 *  control plate with its grass and what the frame drew (which costs those
 *  frames their timing); without, nothing is read back. With `counts`, every
 *  frame drawn exactly on a tick boundary is counted, and one in
 *  `COUNT_BETWEEN` of the rest: a second client's counts there catch a clock
 *  read a hair off. */
export function createReelRun(
  scene: CaptureScene,
  tickHz: number,
  onDone: (result: ReelResult) => void,
  { plates = false, counts = false }: { plates?: boolean; counts?: boolean } = {},
) {
  const durationMs = scene.reel.shots.reduce((sum, shot) => sum + shot.seconds * 1000, 0);
  const recording = new BenchmarkRecording(durationMs);
  const startTick = Math.round(scene.warm_s * tickHz);
  let endTick = startTick;
  let layout = "";
  let presentedHz = tickHz;
  let warm = false;
  let finished = false;
  let adapter: string | null = null;
  let stats: Parameters<NonNullable<ViewportPilot["attach"]>>[0]["stats"] | null = null;
  let lastStats = 0;
  const subject = { unitAt: (_id: number): ArrayLike<number> | null => null };
  let lastSubject: readonly number[] = [0, 0];
  let tracked: { follow: number; at: [number, number]; now: number } | null = null;
  const capture: PresentationSample[] = [];
  const frames: PresentationFrame[] = [];
  const plated: ReelPlate[] = [];
  const counted: FrameCount[] = [];
  let lens: Lens | null = null;
  const track = (follow: number, now: number): readonly number[] => {
    const live = subject.unitAt(follow);
    if (tracked?.follow !== follow) tracked = live && { follow, at: [live[0], live[1]], now };
    if (!tracked) return lastSubject;
    if (live) {
      const k = 1 - Math.exp(-(now - tracked.now) / 1000 / 0.5);
      tracked.at[0] += (live[0] - tracked.at[0]) * k;
      tracked.at[1] += (live[1] - tracked.at[1]) * k;
    }
    tracked.now = now;
    return tracked.at;
  };
  const finish = (outcome: BenchmarkOutcome) => {
    if (finished) return;
    finished = true;
    onDone({
      scene,
      outcome,
      recording,
      startTick,
      endTick,
      tickHz: presentedHz,
      layout,
      adapter,
      capture,
      frames,
      lens,
      plates: plated,
      counts: counted,
    });
  };
  const sample = (now: number) => sampleReel(scene.reel, recording.elapsed(now) / 1000);
  const pilot: ViewportPilot = {
    plates,
    frameRate: true,
    attach(frame) {
      stats = frame.stats;
      adapter = frame.adapter;
    },
    pose(now) {
      const s = sample(now);
      if (s.follow === null) return s.pose;
      lastSubject = Array.from(track(s.follow, now));
      return {
        ...s.pose,
        target: [lastSubject[0] + s.pose.target[0], lastSubject[1] + s.pose.target[1]],
      };
    },
    frame({ now, cpuMs, camera, simTime, canvas, counts: readCounts, plate: readPlate }) {
      if (!warm || finished) return;
      if (!recording.started) {
        recording.start(now);
        lastStats = now;
        return;
      }
      const pose = {
        target: [camera.target[0], camera.target[1]] as const,
        distance: camera.distance,
        yaw: camera.yaw,
        pitch: camera.pitch,
      };
      const reelMs = recording.elapsed(now);
      recording.frame({ now, cpuMs, camera: pose, intended: pilot.pose(now)!, phase: scene.map });
      const index = frames.length;
      const previousMs = frames.at(-1)?.reelMs ?? 0;
      // Read back now, in the task that drew it, before the canvas presents.
      let plate: string | null = null;
      if (readPlate && plateShot(scene.reel, previousMs / 1000, reelMs / 1000) !== null) {
        plate = plateFile(index);
        const { clumps, ...pictures } = readPlate();
        plated.push({
          file: plate,
          ...pictures,
          grass: clumps.then((rows) =>
            grassCells(
              rows.map((c) => c.root),
              camera,
              canvas,
            ),
          ),
        });
      }
      const boundary = Math.round(simTime * presentedHz) / presentedHz === simTime;
      if (counts && (boundary || index % COUNT_BETWEEN === 0)) {
        const { drawn, models, effects } = readCounts();
        counted.push({ frame: index, models, effects, drawn });
      }
      lens ??= { fovY: camera.fovY, near: camera.near, depth: "reverse-z", far: "infinite" };
      frames.push({
        index,
        reelMs,
        tick: endTick,
        simTime,
        eye: eyePosition(vec3.create(), camera),
        target: [camera.target[0], camera.target[1], camera.target[2]],
        up: screenAxes(camera).up,
        canvas: [canvas[0], canvas[1]],
        plate,
      });
      if (stats && now - lastStats >= 2000) {
        const s = stats();
        recording.sample({
          elapsedMs: recording.elapsed(now),
          phase: scene.map,
          gpu: s.gpu,
          memory: s.memory,
          heapBytes: null,
        });
        lastStats = now;
      }
      if (recording.elapsed(now) >= durationMs)
        finish({ status: "complete", reason: "Full scene complete" });
    },
  };
  const scripted: ScriptedSim = {
    warmTo: startTick,
    onReady(info) {
      layout = info.layout;
      presentedHz = info.tickHz;
    },
    onWarm() {
      warm = true;
    },
    onTick(t) {
      endTick = t.tick;
      recording.tick(t);
    },
    onPublication(p) {
      if (!finished) capture.push({ tick: p.tick, digest: p.digest, publication: p.packed });
    },
  };
  return {
    pilot,
    scripted,
    subject,
    recording,
    durationMs,
    captureSamples: () => capture,
    opacity: (now: number) => sample(now).black,
    cancel: () =>
      finish({ status: "cancelled", reason: "Cancelled before the full reel finished" }),
    fail: (reason: string) => finish({ status: "failed", reason }),
  };
}
