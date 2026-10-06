import { expect, test } from "vitest";
import {
  sampleReel,
  filmCamera,
  validateBackdrop,
  validateReel,
  type MenuReel,
} from "@apps/battle-lab/src/menuReel";

const at = (x: number, y: number, distance = 100, yaw = 0, pitch = 0.5) => ({
  target: [x, y] as [number, number],
  distance,
  yaw,
  pitch,
});

/** Two shots of 10 s, half-second dips at every cut. */
const REEL: MenuReel = {
  fade_s: 0.5,
  shots: [
    { seconds: 10, from: at(0, 0, 100, 0), to: at(100, 0, 50, 1) },
    { seconds: 10, from: at(500, 500), to: at(500, 600) },
  ],
};

test("a shot drifts at a steady pace from its first framing to its last", () => {
  const quarter = sampleReel(REEL, 2.5);
  const half = sampleReel(REEL, 5);
  expect(quarter.pose).toEqual(at(25, 0, 87.5, 0.25));
  expect(half.pose).toEqual(at(50, 0, 75, 0.5));
  expect(half.black).toBe(0);
  expect(half.done).toBe(false);
});

test("the picture dips to black across each cut and opens on the next shot", () => {
  expect(sampleReel(REEL, 0).black).toBe(1);
  expect(sampleReel(REEL, 0.25).black).toBeCloseTo(0.5);
  expect(sampleReel(REEL, 9.75).black).toBeCloseTo(0.5);
  const next = sampleReel(REEL, 10);
  expect(next.black).toBe(1);
  expect(next.pose).toEqual(at(500, 500));
  expect(sampleReel(REEL, 15).pose).toEqual(at(500, 550));
});

test("a tracking shot names the unit it follows and frames by an offset from it", () => {
  const tracking: MenuReel = {
    fade_s: 0.5,
    shots: [{ seconds: 10, follow: 7, from: at(-10, 0), to: at(10, 0) }],
  };
  const sample = sampleReel(tracking, 5);
  expect(sample.follow).toBe(7);
  expect(sample.pose.target).toEqual([0, 0]);
  expect(sampleReel(REEL, 5).follow).toBeNull();
});

test("a reel is admitted only when every shot can be drawn and its dips fit inside it", () => {
  const json = JSON.parse(JSON.stringify(REEL));
  expect(validateReel(json)).toEqual(REEL);
  const broken = (edit: (reel: typeof json) => void) => {
    const reel = structuredClone(json);
    edit(reel);
    return () => validateReel(reel);
  };
  expect(broken((r) => (r.shots[1].to.target = [1]))).toThrow(/shots\[1\]\.to/);
  expect(broken((r) => (r.shots[0].from.yaw = Number.NaN))).toThrow(/shots\[0\]\.from/);
  expect(broken((r) => (r.shots[0].seconds = 0.9))).toThrow(/shots\[0\]\.seconds/);
  expect(broken((r) => (r.shots = []))).toThrow(/shots/);
});

test("after the last shot the reel is done and holds black on its last framing", () => {
  const after = sampleReel(REEL, 25);
  expect(after).toEqual({
    pose: at(500, 600),
    follow: null,
    black: 1,
    done: true,
  });
});

test("a backdrop is scenes in order, each a saved battle and its own reel; none is refused", () => {
  const scene = (map: string) => ({
    map,
    encounter: "e",
    seed: 1,
    warm_s: 2,
    reel: { fade_s: 0.5, shots: [{ seconds: 4, from: at(0, 0), to: at(1, 1) }] },
  });
  const backdrop = validateBackdrop({ scenes: [scene("town"), scene("city")] });
  expect(backdrop.scenes.map((s) => [s.map, s.reel.shots.length])).toEqual([
    ["town", 1],
    ["city", 1],
  ]);
  expect(() => validateBackdrop({ scenes: [] })).toThrow(/scenes/);
  expect(() => validateBackdrop({ scenes: [{ ...scene("x"), map: "" }] })).toThrow(
    /scenes\[0\]\.map/,
  );
});

test("the backdrop's camera reaches as close as its closest framing, and is the player's otherwise", () => {
  const player = {
    zoom_min: 25,
    zoom_max: 2000,
    pitch_curve: [
      [25, 0.22],
      [65, 0.85],
    ] as [number, number][],
  };
  const reel = (d: number) => ({
    fade_s: 0.5,
    shots: [{ seconds: 4, from: at(0, 0, 40), to: at(1, 1, d) }],
  });
  const near = filmCamera(player, [reel(40), reel(12)]);
  expect(near.zoom_min).toBe(12);
  // The curve still covers the range: the closest framing pitches as the player's nearest.
  expect(near.pitch_curve).toEqual([
    [12, 0.22],
    [25, 0.22],
    [65, 0.85],
  ]);
  expect(near.zoom_max).toBe(2000);
  expect(filmCamera(player, [reel(30)])).toBe(player);
});
