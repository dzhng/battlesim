// @vitest-environment node
// The pose driver seam: feed frames (what the simulation published) in, one
// pose per soldier and one articulation per vehicle out. Per soldier, never
// per formation.
import { expect, test } from "vitest";
import {
  GAIT,
  PoseDriver,
  type FeedFrame,
  type FeedUnit,
} from "@packages/battle-renderer/src/models/poseDriver";

const CLIPS: Record<string, { duration: number; loop: boolean; stride_m: number | null }> = {
  idle: { duration: 2, loop: true, stride_m: null },
  walk: { duration: 1, loop: true, stride_m: 1.4 },
  run: { duration: 0.8, loop: true, stride_m: 3 },
  kneel_fire: { duration: 1, loop: true, stride_m: null },
  prone_pinned: { duration: 2, loop: true, stride_m: null },
  death: { duration: 2, loop: false, stride_m: null },
};

const driver = () =>
  new PoseDriver({
    mounts: { rifle: ["hand"], tank: ["gun", "hmg"] },
    clip: (name) => CLIPS[name] ?? null,
    halfTrack: { tank: 1.5 },
  });

const squad = (
  soldiers: { id: number; x: number; y: number; posture?: "stand" | "kneel" | "prone" }[],
  extra: Partial<FeedUnit> = {},
): FeedUnit => ({
  id: 1,
  kind: "rifle",
  position: [0, 0, 0],
  yaw: 0,
  soldiers: soldiers.map((s) => ({ id: s.id, position: [s.x, s.y, 0], posture: s.posture })),
  mounts: [{ bearing: 0, elevation: 0, shots: 0 }],
  deployment: null,
  suppression: 0,
  ...extra,
});

const frame = (time: number, units: FeedUnit[], fallen: FeedFrame["fallen"] = []): FeedFrame => ({
  time,
  units,
  fallen,
});

test("each soldier's gait and facing come from his own motion, not the squad's", () => {
  const d = driver();
  d.update(
    frame(0, [
      squad([
        { id: 1, x: 0, y: 0 },
        { id: 2, x: 0, y: 5 },
        { id: 3, x: 0, y: 9 },
      ]),
    ]),
  );
  // Over 1 s: soldier 1 walks east, soldier 2 runs north, soldier 3 stands.
  const out = d.update(
    frame(1, [
      squad(
        [
          { id: 1, x: 1.2, y: 0 },
          { id: 2, x: 0, y: 8 },
          { id: 3, x: 0, y: 9 },
        ],
        { yaw: Math.PI },
      ),
    ]),
  );
  const by = new Map(out.soldiers.map((s) => [s.soldier, s]));
  expect(by.get(1)!.clip).toBe("walk");
  expect(by.get(2)!.clip).toBe("run");
  expect(by.get(3)!.clip).toBe("idle");
  // Turning is rate-limited but heads toward each soldier's own velocity.
  expect(by.get(1)!.facing).toBeCloseTo(0, 5);
  expect(by.get(2)!.facing).toBeCloseTo(Math.PI / 2, 5);
});

test("walking advances phase by ground covered over the clip's stride", () => {
  const d = driver();
  d.update(frame(0, [squad([{ id: 1, x: 0, y: 0 }])]));
  d.update(frame(0.25, [squad([{ id: 1, x: 0.35, y: 0 }])]));
  const out = d.update(frame(0.5, [squad([{ id: 1, x: 0.7, y: 0 }])]));
  expect(out.soldiers[0].clip).toBe("walk");
  expect(out.soldiers[0].phase).toBeCloseTo(0.5, 5); // 0.7 m of a 1.4 m stride since the switch
});

test("a clip change crossfades from the previous clip", () => {
  const d = driver();
  d.update(frame(0, [squad([{ id: 1, x: 0, y: 0 }])]));
  const out = d.update(frame(0.1, [squad([{ id: 1, x: 0.2, y: 0 }])]));
  expect(out.soldiers[0].clip).toBe("walk");
  expect(out.soldiers[0].blend?.clip).toBe("idle");
  expect(out.soldiers[0].blend!.weight).toBeCloseTo(1 - 0.1 / GAIT.fade, 5);
  const later = d.update(frame(0.6, [squad([{ id: 1, x: 0.9, y: 0 }])]));
  expect(later.soldiers[0].blend).toBeNull();
});

test("a shot kneels a still soldier, suppression pins him, and a soldier's own posture wins", () => {
  const d = driver();
  d.update(
    frame(0, [
      squad([
        { id: 1, x: 0, y: 0 },
        { id: 2, x: 3, y: 0, posture: "prone" },
      ]),
    ]),
  );
  const fired = d.update(
    frame(0.5, [
      squad(
        [
          { id: 1, x: 0, y: 0 },
          { id: 2, x: 3, y: 0, posture: "prone" },
        ],
        {
          mounts: [{ bearing: 1, elevation: 0, shots: 3 }],
        },
      ),
    ]),
  );
  expect(fired.soldiers.map((s) => s.clip)).toEqual(["kneel_fire", "prone_pinned"]);
  const pinned = d.update(
    frame(3, [
      squad(
        [
          { id: 1, x: 0, y: 0 },
          { id: 2, x: 3, y: 0, posture: "kneel" },
        ],
        {
          mounts: [{ bearing: 1, elevation: 0, shots: 3 }],
          suppression: 0.9,
        },
      ),
    ]),
  );
  expect(pinned.soldiers.map((s) => s.clip)).toEqual(["prone_pinned", "kneel_fire"]);
});

test("a fallen soldier plays his death once, then lies at its end", () => {
  const d = driver();
  d.update(frame(0, [squad([{ id: 1, x: 0, y: 0 }])]));
  const fell = d.update(
    frame(1, [squad([])], [{ soldier: 1, position: [0, 0, 0], yaw: 2, kind: "rifle" }]),
  );
  expect(fell.soldiers[0]).toMatchObject({ soldier: 1, clip: "death", phase: 0, facing: 2 });
  const later = d.update(
    frame(2, [squad([])], [{ soldier: 1, position: [0, 0, 0], yaw: 2, kind: "rifle" }]),
  );
  expect(later.soldiers[0].phase).toBeCloseTo(0.5, 5);
  // Seen only once already down: a corpse.
  const fresh = driver().update(
    frame(9, [], [{ soldier: 7, position: [1, 1, 0], yaw: 0, kind: "at" }]),
  );
  expect(fresh.soldiers[0]).toMatchObject({ clip: "death", phase: 1, kind: "at" });
});

const tank = (x: number, yaw: number, bearing: number, hmg: number, elevation = 0): FeedUnit => ({
  id: 5,
  kind: "tank",
  position: [x, 0, 0],
  yaw,
  soldiers: [],
  mounts: [
    { bearing, elevation, shots: 0 },
    { bearing: hmg, elevation: 0.2, shots: 0 },
  ],
  deployment: null,
  suppression: 0,
});

test("a tank's turret and HMG are posed relative to what carries them", () => {
  const d = driver();
  const out = d.update(frame(0, [tank(0, 0.5, 1.5, 1.0, 0.9)]));
  const a = out.vehicles[0].articulation;
  expect(a.turret_yaw).toBeCloseTo(1, 6);
  expect(a.hmg_yaw).toBeCloseTo(-0.5, 6);
  expect(a.gun_pitch).toBeCloseTo((20 * Math.PI) / 180, 6); // clamped to the gun's limit
  expect(a.hmg_pitch).toBeCloseTo(0.2, 6);
});

test("driving rolls both tracks; turning in place counter-rotates them", () => {
  const d = driver();
  d.update(frame(0, [tank(0, 0, 0, 0)]));
  const drove = d.update(frame(1, [tank(4, 0, 0, 0)])).vehicles[0].articulation;
  expect([drove.travel_l, drove.travel_r]).toEqual([4, 4]);
  const turned = d.update(frame(2, [tank(4, 0.2, 0, 0)])).vehicles[0].articulation;
  expect(turned.travel_l).toBeCloseTo(4 - 0.2 * 1.5, 6);
  expect(turned.travel_r).toBeCloseTo(4 + 0.2 * 1.5, 6);
});

test("a supply vehicle's deploy progress is its articulation's", () => {
  const out = driver().update(
    frame(0, [{ ...tank(0, 0, 0, 0), kind: "supply", mounts: [], deployment: 0.4 }]),
  );
  expect(out.vehicles[0].articulation.deploy).toBe(0.4);
});
