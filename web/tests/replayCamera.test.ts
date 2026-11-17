// @vitest-environment node
import { expect, test } from "vitest";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  recordCamera,
  replayCameraAt,
  validateReplayCamera,
  type ReplayCameraTrack,
} from "@apps/battle-lab/src/replayCamera";

const camera = (x: number, yaw = 0): Camera3DParams => ({
  target: [x, 20, 3],
  distance: 100,
  yaw,
  pitch: 0.7,
  fovY: 0.8,
  near: 0.5,
  aspect: 1.6,
});

test("following interpolates zoom and rotation across the yaw seam, and an absent track stays free", () => {
  const a = camera(0, Math.PI - 0.1);
  const b = { ...camera(10, -Math.PI + 0.1), distance: 200, pitch: 1.1 };
  const track: ReplayCameraTrack = {
    player: "human",
    side: "blue",
    samples: [
      { time: 2, camera: a },
      { time: 4, camera: b },
    ],
  };
  const view = camera(-100);
  replayCameraAt(view, track, 0);
  expect(view.target).toEqual(a.target);
  replayCameraAt(view, track, 3);
  expect(view.target).toEqual([5, 20, 3]);
  expect(view.distance).toBe(150);
  expect(view.pitch).toBeCloseTo(0.9);
  expect(view.yaw).toBeCloseTo(Math.PI);
  expect(replayCameraAt(view, { ...track, samples: [] }, 3)).toBe(false);
});

test("import refuses invalid camera coordinates and unordered timestamps before drawing", () => {
  const track: ReplayCameraTrack = {
    player: "human",
    side: "blue",
    samples: [
      { time: 2, camera: camera(0) },
      { time: 1, camera: camera(10) },
    ],
  };
  expect(() => validateReplayCamera(track)).toThrow("invalid replay camera sample");
  track.samples[1].time = 3;
  track.samples[1].camera.target[0] = NaN;
  expect(() => validateReplayCamera(track)).toThrow("invalid replay camera sample");
});

test("a serialized camera track holds still until the human moves, then follows battle time", () => {
  const track: ReplayCameraTrack = { player: "human", side: "blue", samples: [] };
  recordCamera(track, 0, camera(0));
  for (let time = 1; time <= 30; time++) recordCamera(track, time, camera(0));
  recordCamera(track, 31, camera(10));
  const loaded = JSON.parse(JSON.stringify(track)) as ReplayCameraTrack;
  const view = camera(-100);
  expect(replayCameraAt(view, loaded, 15)).toBe(true);
  expect(view.target).toEqual([0, 20, 3]);
  replayCameraAt(view, loaded, 30.5);
  expect(view.target).toEqual([5, 20, 3]);
  replayCameraAt(view, loaded, 100);
  expect(view.target).toEqual([10, 20, 3]);
  // Seeking/restarting uses battle time, never a wall-clock cursor.
  replayCameraAt(view, loaded, 0);
  expect(view.target).toEqual([0, 20, 3]);
  expect(loaded.samples.length).toBeLessThan(10);
});
