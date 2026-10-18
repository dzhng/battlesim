import { expect, test } from "vitest";
import {
  sampleReel,
  stillReel,
  plateShot,
  filmCamera,
  validateCapture,
  validateReel,
  type Reel,
} from "@apps/battle-lab/src/capture/capture";
import { reelPilot } from "@apps/battle-lab/src/capture/reelPilot";
import { listMaps, type MapEntry } from "@web/maps/catalogue";

/** A catalogue listing of `capture` maps and `test` maps, by id. */
const listing = (capture: string[], test: string[] = []): MapEntry[] =>
  [
    ...capture.map((id) => [id, "capture"] as const),
    ...test.map((id) => [id, "test"] as const),
  ].map(([id, category]) => ({
    id,
    category,
    status: "released",
    label: id,
    character: "arena",
    biome: "summer",
    size_m: [100, 100],
    tags: [],
    source: "authored",
    seed: null,
    encounters: ["e"],
    benchmarks: [],
  }));

const at = (x: number, y: number, distance = 100, yaw = 0, pitch = 0.5) => ({
  target: [x, y] as [number, number],
  distance,
  yaw,
  pitch,
});

/** Two shots of 10 s, half-second dips at every cut. */
const REEL: Reel = {
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

test("a control plate is the first frame at or past each shot's middle, where the battle is unveiled", () => {
  // A frame every 0.4 s from the reel's start, with a hitch over the second middle.
  const times = [0, 0.4, 4.8, 5.2, 9.6, 10, 14.4, 16, 16.4, 20];
  const plates = times.map((now, i) => [now, plateShot(REEL, i === 0 ? 0 : times[i - 1], now)]);
  expect(plates.filter(([, shot]) => shot !== null)).toEqual([
    [5.2, 0],
    [16, 1],
  ]);
  for (const [now] of plates.filter(([, shot]) => shot !== null))
    expect(sampleReel(REEL, now!).black).toBe(0);
  expect(plateShot(REEL, 4, 5)).toBe(0);
});

test("a tracking shot names the unit it follows and frames by an offset from it", () => {
  const tracking: Reel = {
    fade_s: 0.5,
    shots: [{ seconds: 10, follow: 7, from: at(-10, 0), to: at(10, 0) }],
  };
  const sample = sampleReel(tracking, 5);
  expect(sample.follow).toBe(7);
  expect(sample.pose.target).toEqual([0, 0]);
  expect(sampleReel(REEL, 5).follow).toBeNull();
});

/** A tracking shot on unit 3, held 4 m south of it, then a world shot. */
const TRACKING: Reel = {
  fade_s: 0.5,
  shots: [
    { seconds: 10, follow: 3, from: at(0, -4), to: at(0, -4) },
    { seconds: 10, from: at(500, 500), to: at(500, 500) },
  ],
};

test("the pilot frames a world shot as the reel samples it at each time", () => {
  const pilot = reelPilot(REEL);
  for (const t of [0, 0.25, 5, 10, 15, 25]) {
    const { pose, black, done } = sampleReel(REEL, t);
    expect(pilot(t, () => null)).toEqual({ pose, black, done });
  }
});

test("a tracking shot opens on its unit and glides after it as it moves", () => {
  const pilot = reelPilot(TRACKING);
  let unit: [number, number] = [100, 50];
  expect(pilot(1, () => unit).pose.target).toEqual([100, 46]);
  unit = [120, 50];
  // Half a second later it has closed most of the gap, not jumped onto it.
  const glided = pilot(1.5, (id) => (id === 3 ? unit : null)).pose.target;
  expect(glided[0]).toBeGreaterThan(110);
  expect(glided[0]).toBeLessThan(120);
  // A few seconds on it stands over the unit again.
  expect(pilot(5, () => unit).pose.target[0]).toBeCloseTo(120, 1);
  // The world shot after it ignores the unit.
  expect(pilot(15, () => unit).pose.target).toEqual([500, 500]);
});

test("a followed unit that is gone leaves the camera where it last stood", () => {
  const pilot = reelPilot(TRACKING);
  pilot(1, () => [100, 50]);
  const last = pilot(6, () => [140, 50]).pose.target;
  expect(pilot(7, () => null).pose.target).toEqual(last);
  expect(pilot(9, () => null).pose.target).toEqual(last);
  // Gone before its shot opens: the shot frames where the last subject was.
  const late = reelPilot({ fade_s: 0.5, shots: [TRACKING.shots[1], TRACKING.shots[0]] });
  late(5, () => null);
  expect(late(12, () => null).pose.target).toEqual([500, 496]);
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

test("a reel is refused an endless duration or a framing at no distance, by the field at fault", () => {
  const json = JSON.parse(JSON.stringify(REEL));
  const broken = (edit: (reel: typeof json) => void) => {
    const reel = structuredClone(json);
    edit(reel);
    return () => validateReel(reel);
  };
  expect(broken((r) => (r.fade_s = Infinity))).toThrow(/reel\.fade_s/);
  expect(broken((r) => (r.shots[0].seconds = Infinity))).toThrow(/shots\[0\]\.seconds/);
  expect(
    broken((r) => {
      r.shots[0].seconds = Infinity;
      r.shots[0].zoomDuration = Infinity;
    }),
  ).toThrow(/shots\[0\]\.seconds/);
  expect(broken((r) => (r.shots[0].zoomDuration = Infinity))).toThrow(/shots\[0\]\.zoomDuration/);
  expect(broken((r) => (r.shots[1].to.distance = 0))).toThrow(/shots\[1\]\.to\.distance/);
  expect(broken((r) => (r.shots[0].from.distance = -5))).toThrow(/shots\[0\]\.from\.distance/);
});

test("under reduced motion the reel holds its first framing for its length, on the unit it follows", () => {
  const still = stillReel(TRACKING);
  expect(sampleReel(still, 19.9)).toEqual({
    ...sampleReel(TRACKING, 0),
    black: expect.any(Number),
  });
  expect(sampleReel(still, 20).done).toBe(true);
  // Framed by its offset from the unit, not at the map's origin.
  expect(reelPilot(still)(5, () => [100, 50]).pose.target).toEqual([100, 46]);
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

test("a capture is scenes in order, each a saved battle and its own reel; none is refused", () => {
  const scene = (map: string) => ({
    map,
    encounter: "e",
    seed: 1,
    warm_s: 2,
    reel: { fade_s: 0.5, shots: [{ seconds: 4, from: at(0, 0), to: at(1, 1) }] },
  });
  const maps = listing(["town", "city", "x"]);
  const capture = validateCapture({ scenes: [scene("town"), scene("city")] }, maps);
  expect(capture.scenes.map((s) => [s.map, s.reel.shots.length])).toEqual([
    ["town", 1],
    ["city", 1],
  ]);
  expect(() => validateCapture({ scenes: [] }, maps)).toThrow(/scenes/);
  expect(() => validateCapture({ scenes: [{ ...scene("x"), map: "" }] }, maps)).toThrow(
    /scenes\[0\]\.map/,
  );
  expect(() => validateCapture({ scenes: [{ ...scene("x"), warm_s: Infinity }] }, maps)).toThrow(
    /scenes\[0\]\.warm_s/,
  );
});

test("a capture films only capture maps: a test map is refused by name", () => {
  const scene = (map: string) => ({
    map,
    encounter: "e",
    seed: 1,
    warm_s: 2,
    reel: { fade_s: 0.5, shots: [{ seconds: 4, from: at(0, 0), to: at(1, 1) }] },
  });
  const maps = listing(["town"], ["range"]);
  expect(() => validateCapture({ scenes: [scene("town"), scene("range")] }, maps)).toThrow(
    'capture scenes[1].map: the saved map "range" is a test map, not a capture map',
  );
  expect(() => validateCapture({ scenes: [scene("nowhere")] }, maps)).toThrow(
    /scenes\[0\]\.map: no saved map "nowhere"/,
  );
});

test("a capture's camera reaches as close as its closest framing, and is the player's otherwise", () => {
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

test("a shot keeps its easing, log-distance zoom and zoom duration", () => {
  const shot = {
    seconds: 10,
    follow: 3,
    from: at(0, 0, 700),
    to: at(0, 0, 40),
    ease: "cubic",
    logDistance: true,
    zoomDuration: 8,
  };
  expect(validateReel({ fade_s: 0, shots: [shot] }).shots[0]).toEqual(shot);
  const refused = (edit: Record<string, unknown>) => () =>
    validateReel({ fade_s: 0, shots: [{ ...shot, ...edit }] });
  expect(refused({ ease: "bounce" })).toThrow("capture reel.shots[0].ease");
  expect(refused({ logDistance: "yes" })).toThrow("capture reel.shots[0].logDistance");
  expect(refused({ zoomDuration: 11 })).toThrow("capture reel.shots[0].zoomDuration");
  expect(refused({ zoomDuration: 0 })).toThrow("capture reel.shots[0].zoomDuration");
});

test("a field a capture doesn't have is refused by its path", () => {
  const scene = {
    map: "town",
    encounter: "e",
    seed: 1,
    warm_s: 2,
    reel: { fade_s: 0.5, shots: [{ seconds: 4, from: at(0, 0), to: at(1, 1) }] },
  };
  const maps = listing(["town"]);
  const refused = (edit: (c: { scenes: (typeof scene)[] } & Record<string, unknown>) => void) => {
    const capture = structuredClone({ scenes: [scene] }) as Parameters<typeof edit>[0];
    edit(capture);
    return () => validateCapture(capture, maps);
  };
  expect(refused((c) => (c.loop = true))).toThrow("capture loop: not a capture field");
  expect(refused((c) => Object.assign(c.scenes[0], { units: [] }))).toThrow(
    "capture scenes[0].units: not a capture field",
  );
  expect(refused((c) => Object.assign(c.scenes[0].reel, { speed: 2 }))).toThrow(
    "capture scenes[0].reel.speed: not a capture field",
  );
  expect(refused((c) => Object.assign(c.scenes[0].reel.shots[0], { fov: 1 }))).toThrow(
    "capture scenes[0].reel.shots[0].fov: not a capture field",
  );
  expect(refused((c) => Object.assign(c.scenes[0].reel.shots[0].to, { roll: 0 }))).toThrow(
    "capture scenes[0].reel.shots[0].to.roll: not a capture field",
  );
});

test("every capture file validates against the map catalogue", () => {
  const files = import.meta.glob<unknown>("../../fixtures/captures/*.json", {
    eager: true,
    import: "default",
  });
  const names = Object.keys(files).map((path) => path.split("/").at(-1));
  expect(names).toEqual(
    expect.arrayContaining([
      "menu.json",
      "default.json",
      "top-down.json",
      "ground-level.json",
      "orbit.json",
    ]),
  );
  for (const [path, json] of Object.entries(files))
    expect(() => validateCapture(json, listMaps()), path).not.toThrow();
});
