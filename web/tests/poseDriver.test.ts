// @vitest-environment node
// The pose driver seam: feed frames (what the simulation published) in, one
// pose per soldier and one articulation per vehicle out. Per soldier, never
// per formation.
import village from "@fixtures/village.json";
import { expect, test } from "vitest";
import type { Vec3 } from "math";
import {
  loopStart,
  PoseDriver,
  restManner,
  validatePoseFeel,
  type FeedFrame,
  type FeedUnit,
} from "@packages/battle-renderer/src/models/poseDriver";
import { villagePose as FEEL } from "@apps/battle-lab/src/poseFeed";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { shippedMounts } from "./shippedMounts";

const REST = FEEL.rest;

const CLIPS: Record<string, { duration: number; loop: boolean; stride_m: number | null }> = {
  idle: { duration: 2, loop: true, stride_m: null },
  walk: { duration: 1, loop: true, stride_m: 1.4 },
  run: { duration: 0.8, loop: true, stride_m: 3 },
  kneel_fire: { duration: 1, loop: true, stride_m: null },
  prone_pinned: { duration: 2, loop: true, stride_m: null },
  death: { duration: 2, loop: false, stride_m: null },
};

/** Half the tank's track gauge: its hull's half width by its gauge share. */
const HALF_TRACK = UNITS.hull("tank")!.half_extents_m[1] * (FEEL.gauge.tank ?? 1);

const driver = () =>
  new PoseDriver({
    units: UNITS,
    mounts: shippedMounts,
    clip: (_kind, name) => CLIPS[name] ?? null,
    pinned: 0.85,
    feel: FEEL,
    leanHold: village.cover.lean_hold_s,
  });

const squad = (
  soldiers: { id: number; x: number; y: number; posture?: "stand" | "kneel" | "prone" }[],
  extra: Partial<FeedUnit> = {},
): FeedUnit => ({
  id: 1,
  kind: "rifle",
  side: "blue",
  position: [0, 0, 0],
  yaw: 0,
  soldiers: soldiers.map((s) => ({
    id: s.id,
    slot: 0,
    position: [s.x, s.y, 0],
    posture: s.posture,
  })),
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
  // 0.7 m of a 1.4 m stride since the switch, from his own starting phase.
  expect(out.soldiers[0].phase).toBeCloseTo((loopStart(1) + 0.5) % 1, 5);
});

test("soldiers who start a loop together are out of step, each by his own offset", () => {
  const d = driver();
  const men = [1, 2, 3, 4].map((id) => ({ id, x: id * 2, y: 0 }));
  d.update(frame(0, [squad(men)]));
  const walked = d.update(frame(0.5, [squad(men.map((m) => ({ ...m, y: 0.6 })))]));
  expect(walked.soldiers.every((s) => s.clip === "walk")).toBe(true);
  const phases = walked.soldiers.map((s) => s.phase);
  expect(new Set(phases.map((p) => p.toFixed(2))).size).toBe(4);
  // The same soldier always starts at the same place: replays are stable.
  expect(loopStart(3)).toBe(loopStart(3));
});

test("a clip change crossfades from the previous clip", () => {
  const d = driver();
  d.update(frame(0, [squad([{ id: 1, x: 0, y: 0 }])]));
  const out = d.update(frame(0.1, [squad([{ id: 1, x: 0.2, y: 0 }])]));
  expect(out.soldiers[0].clip).toBe("walk");
  expect(out.soldiers[0].blend?.clip).toBe("idle");
  expect(out.soldiers[0].blend!.weight).toBeCloseTo(1 - 0.1 / FEEL.gait.fade_s, 5);
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

test("a fallen soldier plays his death once, facing as he fell, then lies static", () => {
  const d = driver();
  d.update(frame(0, [squad([{ id: 1, x: 0, y: 0 }])]));
  const fallen = [
    {
      soldier: 1,
      position: [0, 0, 0] as Vec3,
      yaw: 2,
      kind: "rifle" as const,
      slot: 0,
      side: "blue" as const,
    },
  ];
  const fell = d.update(frame(1, [squad([])], fallen));
  // His own facing (the squad's heading, strayed by his manner at rest), not
  // the published yaw of his fall.
  const own = restManner(1, REST).turn;
  expect(fell.soldiers[0]).toMatchObject({ soldier: 1, clip: "death", phase: 0 });
  expect(fell.soldiers[0].facing).toBeCloseTo(own, 9);
  expect(fell.corpses).toEqual([]);
  const later = d.update(frame(2, [squad([])], fallen));
  expect(later.soldiers[0].phase).toBeCloseTo(0.5, 5);
  // Played out: no longer a posed body, but a static corpse where he fell.
  const done = d.update(frame(3.1, [squad([])], fallen));
  expect(done.soldiers).toEqual([]);
  expect(done.corpses).toEqual([
    { soldier: 1, kind: "rifle", slot: 0, side: "blue", position: [0, 0, 0], yaw: own },
  ]);
});

test("a squad at rest looks different ways and idles out of step; shooting, every man faces the aim", () => {
  const d = driver();
  const men = [1, 2, 3, 4, 5, 6, 7, 8].map((id) => ({ id, x: id * 2, y: 0 }));
  d.update(frame(0, [squad(men)]));
  const rest = d.update(frame(3, [squad(men)]));
  // At ease or watching, weapon up: both stances in one squad, each man keeping his own.
  const stances = rest.soldiers.map((s) => s.clip);
  expect(stances).toEqual(
    rest.soldiers.map((s) => (restManner(s.soldier, REST).watch ? "stand_aim" : "idle")),
  );
  expect(new Set(stances)).toEqual(new Set(["idle", "stand_aim"]));
  const facings = rest.soldiers.map((s) => s.facing);
  // Each within his own stray of the squad's heading (0), and no two alike.
  expect(facings.every((f) => Math.abs(f) <= REST.turn_rad)).toBe(true);
  expect(new Set(facings.map((f) => f.toFixed(2))).size).toBe(men.length);
  expect(Math.max(...facings) - Math.min(...facings)).toBeGreaterThan(REST.turn_rad);
  // Idles advance at each man's own tempo: after 3 s their phases have drifted
  // apart by more than their starting offsets explain.
  const drift = rest.soldiers.map((s) => (s.phase - loopStart(s.soldier) + 1) % 1);
  expect(new Set(drift.map((p) => p.toFixed(2))).size).toBeGreaterThan(4);
  // The same man always stands the same way: replays are stable.
  expect(restManner(5, REST)).toEqual(restManner(5, REST));

  const aimed = (time: number) =>
    d.update(frame(time, [squad(men, { mounts: [{ bearing: 1.2, elevation: 0, shots: 3 }] })]));
  aimed(3.1);
  for (const s of aimed(4).soldiers) expect(s.facing).toBeCloseTo(1.2, 5);
  // Quiet again for longer than the settle: each man's gaze strays once more.
  const quiet = aimed(3.1 + REST.settle_s[0] + REST.settle_s[1] + 1).soldiers;
  for (const s of quiet) expect(s.facing).toBeCloseTo(1.2 + restManner(s.soldier, REST).turn, 5);
});

test("a soldier slides out to his lean point while he fires, then eases back in, never walking", () => {
  const d = driver();
  const at = (lean: [number, number] | null) =>
    squad([{ id: 1, x: 0, y: 0 }], {
      soldiers: [{ id: 1, slot: 0, position: [0, 0, 0], lean }],
    });
  const x = (time: number, lean: [number, number] | null) => {
    const s = d.update(frame(time, [at(lean)])).soldiers[0];
    expect(["walk", "run"]).not.toContain(s.clip);
    return s.position[0];
  };
  x(0, null);
  // Out: partway at first, all the way once the slide is done.
  const first = x(0.1, [0.8, 0]);
  expect(first).toBeGreaterThan(0);
  expect(first).toBeLessThan(0.8);
  expect(x(0.1 + FEEL.lean.out_s, [0.8, 0])).toBeCloseTo(0.8, 5);
  expect(x(1.5, [0.8, 0])).toBeCloseTo(0.8, 5);
  // Tucked in again: eased back, not snapped.
  const back = x(1.6, null);
  expect(back).toBeGreaterThan(0);
  expect(back).toBeLessThan(0.8);
  expect(x(1.6 + FEEL.lean.back_s, null)).toBeCloseTo(0, 5);
});

test("a pinned soldier lies behind his cover, and kneels to fire out on his lean", () => {
  const d = driver();
  const pinned = (lean: [number, number] | null) =>
    squad([{ id: 1, x: 0, y: 0 }], {
      soldiers: [{ id: 1, slot: 0, position: [0, 0, 0], lean }],
      suppression: 0.9,
    });
  d.update(frame(0, [pinned(null)]));
  expect(d.update(frame(1, [pinned(null)])).soldiers[0].clip).toBe("prone_pinned");
  expect(d.update(frame(1.5, [pinned([0.8, 0])])).soldiers[0].clip).toBe("kneel_fire");
  expect(d.update(frame(4, [pinned(null)])).soldiers[0].clip).toBe("prone_pinned");
});

const tank = (x: number, yaw: number, bearing: number, hmg: number, elevation = 0): FeedUnit => ({
  id: 5,
  kind: "tank",
  side: "blue",
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
  expect(turned.travel_l).toBeCloseTo(4 - 0.2 * HALF_TRACK, 6);
  expect(turned.travel_r).toBeCloseTo(4 + 0.2 * HALF_TRACK, 6);
});

test("a supply vehicle's deploy progress is its articulation's", () => {
  const out = driver().update(
    frame(0, [{ ...tank(0, 0, 0, 0), kind: "supply", mounts: [], deployment: 0.4 }]),
  );
  expect(out.vehicles[0].articulation.deploy).toBe(0.4);
});

test("presentation.pose is checked: a run no faster than a walk, or a gauge past the hull, is refused", () => {
  expect(validatePoseFeel(FEEL)).toBe(FEEL);
  expect(() =>
    validatePoseFeel({ ...FEEL, gait: { ...FEEL.gait, run_mps: FEEL.gait.walk_mps } }),
  ).toThrow(/gait\.run_mps/);
  expect(() => validatePoseFeel({ ...FEEL, gauge: { tank: 1.2 } })).toThrow(/gauge\.tank/);
});
