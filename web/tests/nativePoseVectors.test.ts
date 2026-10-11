// @vitest-environment node
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { expect, test } from "vitest";
import game from "@fixtures/game.json";
import type { ObservationLayout } from "@web/battle/sim/observationView";
import { ObservationDecoder, loadPresentation } from "@web/battle/sim/presentation";
import { TickInterpolator } from "@web/battle/present/interpolate";
import { nodeCatalogSet } from "@web/battle/catalog/node";
import { unitArtifacts, type UnitsManifest } from "@web/battle/benchmark/unitManifest";
import { AppearanceLibrary } from "@packages/scene-assets/src/loader.ts";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog.ts";
import {
  articulate,
  articulationRig,
  restLocals,
  REST_ARTICULATION,
} from "@packages/scene-assets/src/articulation.ts";
import { poseWorlds } from "@packages/scene-assets/src/pose.ts";
import { modelCounts } from "@packages/battle-renderer/src/models/modelInstances";
import { createPoseDriver, gamePose, ObservationFeed } from "@apps/battle-lab/src/poseFeed";
import { describeUnits } from "@apps/battle-lab/src/benchmark/unitPresentation";

await loadPresentation();

// Writes the browser's poses for native's fixture capture, the vectors
// native's port of the per-frame unit derivations (`native/godot/units/`) is
// compared against (`pose_vectors_test.gd`):
// - every frame record of paris-corner up to its first plate, driven through
//   the browser's own `TickInterpolator`, `ObservationFeed` and `PoseDriver`
//   as the battle session drives them, with the poses of every sampled record;
// - the tank's `articulate` locals at fixed inputs (turret bearing, gun
//   pitch and recoil, wheel travel);
// - a rifleman's joint worlds at fixed clips and phases, crossfades included.
// It also writes the `units` section those frames were posed with, which the
// fixture run's manifest must carry (the native test checks), and its clip
// tables beside the fixture run's manifest. Rerun it when the pose driver,
// the feed, articulation or the units' art changes.
const fixtures = new URL("../../native/godot/tests/fixtures/", import.meta.url);
const RUNTIME = new URL("../../assets/runtime/", import.meta.url);
/** Every this many records a frame's poses are written, and the last. */
const EVERY = 15;

const round = (v: number) => Math.round(v * 1e6) / 1e6;
const point = (p: ArrayLike<number>) => Array.from(p, round);

test("the browser's poses of native's fixture capture are written for native", async () => {
  const capture = JSON.parse(
    readFileSync(new URL("run/paris-corner/capture.json", fixtures), "utf8"),
  );
  const decoder = new ObservationDecoder(JSON.parse(capture.layout) as ObservationLayout);
  const observations = capture.samples.map(
    (s: { publication: number[] }) =>
      decoder.decode(new Float32Array(new Uint32Array(s.publication).buffer))!,
  );
  const kinds = [
    ...new Set(
      observations.flatMap((o: { own: { kind: string }[]; identified: { kind: string }[] }) =>
        [...o.own, ...o.identified].map((u) => u.kind),
      ),
    ),
  ] as string[];
  const { units } = await nodeCatalogSet("menu");
  const wearing = new Set<string>();
  for (const kind of kinds) {
    const type = units.type(kind);
    if (type.appearance) wearing.add(type.appearance);
    for (const m of type.mounts)
      for (const n of [
        ...(m.operator_appearance?.active ?? []),
        ...(m.operator_appearance?.carried ?? []),
      ])
        wearing.add(n);
    for (const slot of units.slots(kind))
      for (const n of units.soldier(slot).appearance) wearing.add(n);
  }
  const installed = await new AppearanceLibrary(async (url) => {
    const bytes = readFileSync(new URL(url.replace("mem:/", ""), RUNTIME));
    return {
      ok: true,
      status: 200,
      arrayBuffer: async () =>
        bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer,
      json: async () => JSON.parse(bytes.toString()),
    };
  }).load("mem:/", wearing);
  const leanHold = game.cover.lean_hold_s;
  const drawn = describeUnits({ kinds, installed, units, pose: gamePose, leanHold });
  const sha256 = async (bytes: Uint8Array) => createHash("sha256").update(bytes).digest("hex");
  const { units: section, artifacts } = await unitArtifacts(drawn, sha256);

  // The battle session's chain, on the frame records' clocks: each record
  // draws the publication at its tick, blended from the one before.
  const tickMs = 1000 / capture.tickHz;
  const catalog = new AppearanceCatalog(installed, units);
  const resolve = (...a: Parameters<AppearanceCatalog["resolve"]>) => catalog.resolve(...a);
  const driver = createPoseDriver({ cover: { lean_hold_s: leanHold } }, units, installed);
  const feed = new ObservationFeed(capture.side, units);
  const interpolator = new TickInterpolator(tickMs);
  let pushed = -1;
  const frames: unknown[] = [];
  for (const record of capture.frames) {
    while (pushed + 1 < observations.length && observations[pushed + 1].tick <= record.tick)
      interpolator.push(observations[++pushed], observations[pushed].tick * tickMs);
    const latest = observations[pushed].tick;
    const from = pushed > 0 ? observations[pushed - 1].tick : latest;
    // `now` puts the interpolator's clock at the record's.
    const now = latest * tickMs + (record.simTime * capture.tickHz - from) * tickMs;
    const sample = interpolator.frame(now)!;
    expect(sample.time).toBeCloseTo(record.simTime, 9);
    const poses = driver.update(feed.frame(sample));
    if (record.index % EVERY !== 0 && record.index !== capture.frames.length - 1) continue;
    frames.push({
      index: record.index,
      counts: modelCounts(poses, resolve),
      soldiers: poses.soldiers.map((s) => ({
        soldier: s.soldier,
        unit: s.unit,
        clip: s.clip,
        phase: round(s.phase),
        facing: round(s.facing),
        position: point(s.position),
        blend: s.blend && {
          clip: s.blend.clip,
          phase: round(s.blend.phase),
          weight: round(s.blend.weight),
        },
        operatorMount: s.operatorMount,
        activeMount: s.activeMount,
      })),
      vehicles: poses.vehicles.map((v) => ({
        unit: v.unit,
        position: point(v.position),
        yaw: round(v.yaw),
        articulation: Object.fromEntries(
          Object.entries(v.articulation).map(([k, x]) => [k, round(x)]),
        ),
      })),
      corpses: poses.corpses.map((c) => ({
        soldier: c.soldier,
        lies: c.lies,
        position: point(c.position),
        yaw: round(c.yaw),
      })),
    });
  }

  // `articulate` at fixed inputs, on the fielded vehicle's rig.
  const [vehicle] = Object.entries(section.appearances).find(([, a]) => a.kind === "articulated")!;
  const rigged = installed.appearances.get(vehicle)!.bundle;
  if (rigged.kind !== "articulated") throw new Error("not a vehicle");
  const rig = articulationRig(rigged.nodes);
  const inputs = [
    { ...REST_ARTICULATION },
    { ...REST_ARTICULATION, turret_yaw: 0.7, gun_pitch: 0.2, recoil: 0.3 },
    { ...REST_ARTICULATION, turret_yaw: -2.5, gun_pitch: -0.4, hmg_yaw: 1.1, hmg_pitch: 1.2 },
    { ...REST_ARTICULATION, travel_l: 3.25, travel_r: -1.5 },
  ];
  const articulations = inputs.map((input) => ({
    input,
    locals: articulate(restLocals(rigged.nodes), rigged.nodes, rig, input).map((l) => ({
      t: point(l.t),
      r: point(l.r),
      s: point(l.s),
    })),
  }));

  // A body's joint worlds at fixed clips and phases.
  const [body] = Object.entries(section.appearances).find(([, a]) => a.kind === "skinned")!;
  const skinned = installed.appearances.get(body)!.bundle;
  if (skinned.kind !== "skinned") throw new Error("not a body");
  const skeleton = installed.skeletons.get(skinned.skeleton)!;
  const joints = [
    { clip: "idle", phase: 0 },
    { clip: "walk", phase: 0.37 },
    { clip: "run", phase: 0.9 },
    { clip: "death", phase: 1 },
    { clip: "kneel_fire", phase: 0.5 },
  ].map((pose) => ({
    ...pose,
    worlds: poseWorlds(skinned, skeleton, pose).map((m) => point([m[12], m[13], m[14]])),
  }));

  mkdirSync(new URL("poses/", fixtures), { recursive: true });
  writeFileSync(
    new URL("poses/paris-corner-units.json", fixtures),
    JSON.stringify(section satisfies UnitsManifest) + "\n",
  );
  for (const { file, bytes } of artifacts)
    writeFileSync(new URL(`run/paris-corner/${file}`, fixtures), bytes);
  writeFileSync(
    new URL("poses/paris-corner-poses.json", fixtures),
    JSON.stringify({
      capture: "run/paris-corner/capture.json",
      frames,
      vehicle,
      articulations,
      body,
      joints,
    }) + "\n",
  );
  // The fixture poses soldiers, a vehicle and corpses.
  const last = frames.at(-1) as { soldiers: unknown[]; vehicles: unknown[] };
  expect(last.soldiers.length).toBeGreaterThan(0);
  expect(last.vehicles.length).toBeGreaterThan(0);
}, 120_000);
