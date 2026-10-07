import { fixtureMap } from "../fixtures";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldLayers } from "@packages/battle-renderer/src/worldMesh";
import {
  buildFlightOverlay,
  type FlightBody,
  type FlightMark,
  type FlightTrace,
} from "@packages/battle-renderer/src/flightMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import type { MapDefinition } from "@web/maps/resolve";
import { SavedMap } from "../savedMaps";
import game from "@fixtures/game.json";
import type { WeaponRow } from "@packages/scene-assets/src/units";
import { LabViewport, type ViewportFrame } from "../LabViewport";
import {
  createEffectBatch,
  type EffectBlast,
  type EffectPublication,
  type EffectSegment,
} from "@packages/battle-renderer/src/effects/effectFrame";
import { createEffectFrame, gameEffects } from "../effectFeed";
import { useStandingBuildings, useStaticWorld, type WorldView } from "../useStaticWorld";
import type { GameRules, SessionCatalog } from "@web/battle/catalog/compose";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { gameBiome } from "../gameBiome";
import { useMapAppearances } from "../gameAppearances";
import { loadWasm, type Wasm } from "@web/battle/sim/module";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";

// Flight reproduction bench. Scripted bodies move at constant velocity (one
// reverses after launch); emitters fire the game weapon rows through the
// same solve → spread → launch path weapons use, all at tick 0. Nothing here
// decides a hit: the Rust store reports every event, judging armoured bodies
// by the battle's hull policy, so failed penetrations may glance off.
// Beside the diagnostic traces and marks, the battle's combat effects draw
// each tick as a publication would carry it: every round's stretch, its
// ricochets and impact, blasts, and each emitter's launch.

const SEED = 20260925;
const TICK_HZ = game.tick_hz;
const P = game.physics;

type Xyz = [number, number, number];

interface Mover {
  id: number;
  unit: number;
  shape: "capsule" | "box";
  dims: number[];
  start: [number, number];
  /** Heading of the body's +X, radians. */
  yaw: number;
  velocity: [number, number];
  /** Reverses at this tick: the launch only ever saw the first velocity. */
  reverseAtTick?: number;
  /** A tank hull, judged by the fixture's tank armour. */
  armored?: boolean;
}

interface Shot {
  label: string;
  weapon: object;
  /** The round kind its effects take (a weapon row name). */
  kind: string;
  from: [number, number];
  muzzle: number;
  aim: { body: number; height: number } | { ground: [number, number] };
}

/** The bench's scripted bodies and emitters, from its session's catalog:
 *  the test set's tank and the game's weapon rows. */
interface Bench {
  movers: Mover[];
  shots: Shot[];
  rules: GameRules;
  tankArmor: unknown;
}

function benchOf(catalog: SessionCatalog): Bench {
  const units = catalog.units;
  // Resolved weapon rows, with speed divided by sqrt(gravity_scale) and gravity
  // normalized to one: the same stationary arc at a diagnostic flight speed.
  // Scripted bodies isolate swept collision from gameplay's flight timing.
  const W: Record<string, WeaponRow> = Object.fromEntries(
    Object.entries(catalog.weapons).map(([name, row]) => {
      const g = typeof row.gravity_scale === "number" ? row.gravity_scale : 1;
      return [name, { ...row, speed_mps: row.speed_mps / Math.sqrt(g), gravity_scale: 1 }];
    }),
  );
  // No game weapon has indirect-fire capability yet; this lab-only row
  // exercises the opt-in high arc with a slow round whose apex fits the frame.
  const LAB_MORTAR = {
    speed_mps: 45,
    range_m: 500,
    scatter_mrad: 10,
    suppression_radius_m: 10,
    trajectory: "indirect",
  };
  // The oblique-AP preset: game AP pierces every face of the tank, so this
  // lab-only row is a spent AP round (penetration below the front plate) whose
  // failed penetrations may glance off.
  const LAB_SPENT_AP = { ...W.tank_ap, penetration: 120 };

  const SOLDIER = [P.soldier_radius_m, P.soldier_height_m];
  const PRESET_TANK: [number, number] = [238, 262];
  const TANK = units.hull("tank")!.half_extents_m;
  const NORTH = Math.PI / 2;

  const movers: Mover[] = [
    // Fast bullet vs a 0.3 m thick board sliding across the line.
    {
      id: 1,
      unit: 1,
      shape: "box",
      dims: [0.15, 0.6, 0.9],
      start: [150, 134],
      yaw: 0,
      velocity: [0, 4],
    },
    // Dodge: both walk north; the second reverses ten ticks after launch.
    {
      id: 2,
      unit: 2,
      shape: "capsule",
      dims: SOLDIER,
      start: [150, 96],
      yaw: NORTH,
      velocity: [0, 3],
    },
    {
      id: 3,
      unit: 3,
      shape: "capsule",
      dims: SOLDIER,
      start: [150, 112],
      yaw: NORTH,
      velocity: [0, 3],
      reverseAtTick: 10,
    },
    // Crossing bodies: a soldier crosses the grenade's line early, a tank late.
    {
      id: 4,
      unit: 4,
      shape: "capsule",
      dims: SOLDIER,
      start: [360, 140],
      yaw: NORTH,
      velocity: [0, 3],
    },
    {
      id: 5,
      unit: 5,
      shape: "box",
      dims: TANK,
      start: [380, 140],
      yaw: NORTH,
      velocity: [0, 8],
      armored: true,
    },
    // The oblique-AP preset's target: a standing tank, its front turned 25° off
    // the line of fire from the west.
    {
      id: 6,
      unit: 6,
      shape: "box",
      dims: TANK,
      start: PRESET_TANK,
      yaw: Math.PI + (25 * Math.PI) / 180,
      velocity: [0, 0],
      armored: true,
    },
  ];

  const shots: Shot[] = [
    ...[60, 120, 180].map(
      (range, i): Shot => ({
        label: `grenade arc ${range} m`,
        weapon: W.grenade,
        kind: "grenade",
        from: [20, 14 + 6 * i],
        muzzle: P.infantry_muzzle_m,
        aim: { ground: [20 + range, 14 + 6 * i] },
      }),
    ),
    {
      label: "hmg at sliding board",
      weapon: W.hmg,
      kind: "hmg",
      from: [20, 140],
      muzzle: 2,
      aim: { body: 1, height: 0.9 },
    },
    {
      label: "grenade at walker",
      weapon: W.grenade,
      kind: "grenade",
      from: [20, 96],
      muzzle: P.infantry_muzzle_m,
      aim: { body: 2, height: 0.85 },
    },
    {
      label: "grenade at dodger",
      weapon: W.grenade,
      kind: "grenade",
      from: [20, 112],
      muzzle: P.infantry_muzzle_m,
      aim: { body: 3, height: 0.85 },
    },
    {
      label: "direct grenade over crest",
      weapon: W.grenade,
      kind: "grenade",
      from: [100, 150],
      muzzle: P.infantry_muzzle_m,
      aim: { ground: [100, 292] },
    },
    {
      label: "indirect lab mortar over crest",
      weapon: LAB_MORTAR,
      kind: "grenade",
      from: [92, 118],
      muzzle: P.infantry_muzzle_m,
      aim: { ground: [92, 292] },
    },
    {
      label: "grenade across crossing bodies",
      weapon: W.grenade,
      kind: "grenade",
      from: [250, 140],
      muzzle: P.infantry_muzzle_m,
      aim: { ground: [380, 140] },
    },
    // Oblique AP: spent AP onto the tank's front 25° off its normal, and HMG
    // onto its side about 37° off its normal from the south-east, each round
    // rolling its face's chance.
    ...[0, 1, 2, 3].map(
      (k): Shot => ({
        label: `oblique AP ${k + 1}`,
        weapon: LAB_SPENT_AP,
        kind: "tank_ap",
        from: [PRESET_TANK[0] - 26, PRESET_TANK[1] - 2 + k],
        muzzle: 2,
        // Spread up the plate so each round's mark stands apart.
        aim: { body: 6, height: 0.5 + 0.45 * k },
      }),
    ),
    ...[0, 1, 2, 3].map(
      (k): Shot => ({
        label: `hmg at tank side ${k + 1}`,
        weapon: W.hmg,
        kind: "hmg",
        from: [PRESET_TANK[0] + 16, PRESET_TANK[1] - 9 + 0.6 * k],
        muzzle: 2,
        aim: { body: 6, height: 0.5 + 0.45 * k },
      }),
    ),
  ];
  return { movers, shots, rules: catalog.rules, tankArmor: units.hull("tank")!.armor };
}

const BALLISTICS_CAMERA: Camera3DParams = {
  target: [160, 150, 10],
  distance: 380,
  pitch: 0.62,
  yaw: -1.3,
  ...gameCamera.lens,
};

type Lab = InstanceType<Wasm["FlightLab"]>;

interface ShotResult {
  label: string;
  fired: boolean;
  projectile?: number;
  arc?: string;
  reason?: string;
  intercept?: Xyz;
  blocked_at?: Xyz;
  path?: Xyz[];
  time_of_flight?: number;
}

interface LabEvent {
  tick: number;
  kind: "impact" | "ricochet" | "near_miss" | "expired";
  projectile: number;
  struck?: string;
  unit?: number;
  distance?: number;
  cause?: string;
  point: Xyz;
  /** Outward surface normal at an impact or ricochet. */
  normal?: Xyz;
  /** A ricochet's velocity leaving the hull. */
  deflected?: Xyz;
  /** Ricochets before this event. */
  bounces?: number;
  time: number;
}

/** Events that end a round (a ricochet does not). */
const ends = (e: LabEvent) => e.kind === "impact" || e.kind === "expired";

interface Run {
  lab: Lab;
  tick: number;
  shots: ShotResult[];
  events: LabEvent[];
  paths: Map<number, Xyz[]>;
  movers: Mover[];
  bench: Bench;
}

function poseAt(view: WorldView, m: Mover, tick: number) {
  const dt = 1 / TICK_HZ;
  const turn = m.reverseAtTick ?? Infinity;
  const forward = Math.min(tick, turn) * dt;
  const back = Math.max(0, tick - turn) * dt;
  const x = m.start[0] + m.velocity[0] * (forward - back);
  const y = m.start[1] + m.velocity[1] * (forward - back);
  return { base: [x, y, view.height_at(x, y) ?? 0] as Xyz, yaw: m.yaw };
}

/** Bodies over tick `k` (from its start to its end), packed for the lab. */
function packBodies(view: WorldView, movers: Mover[], k: number): Float64Array {
  const out: number[] = [];
  for (const m of movers) {
    const [a, b] = [poseAt(view, m, k - 1), poseAt(view, m, k)];
    const dims = [...m.dims, 0, 0, 0].slice(0, 3);
    out.push(
      m.id,
      m.unit,
      m.shape === "capsule" ? 0 : m.armored ? 2 : 1,
      ...dims,
      ...a.base,
      a.yaw,
      ...b.base,
      b.yaw,
    );
  }
  return Float64Array.from(out);
}

function startRun(
  wasm: Wasm,
  bench: Bench,
  map: MapDefinition,
  view: WorldView,
  spread: boolean,
): Run {
  const lab = new wasm.FlightLab(
    JSON.stringify(map),
    JSON.stringify(bench.rules),
    JSON.stringify(bench.tankArmor),
    SEED,
  );
  const ground = (xy: [number, number], lift: number): Xyz => [
    xy[0],
    xy[1],
    (view.height_at(xy[0], xy[1]) ?? 0) + lift,
  ];
  const paths = new Map<number, Xyz[]>();
  const shots = bench.shots.map((shot): ShotResult => {
    const origin = ground(shot.from, shot.muzzle);
    let target: Xyz;
    let velocity: Xyz = [0, 0, 0];
    if ("body" in shot.aim) {
      const { body, height } = shot.aim;
      const m = bench.movers.find((b) => b.id === body)!;
      const p = poseAt(view, m, 0);
      target = [p.base[0], p.base[1], p.base[2] + height];
      // Observed now: the launch never learns of a later reversal.
      velocity = [m.velocity[0], m.velocity[1], 0];
    } else {
      target = ground(shot.aim.ground, 0);
    }
    const scatter = spread ? (shot.weapon as { scatter_mrad: number }).scatter_mrad : 0;
    const result = JSON.parse(
      lab.fire(
        JSON.stringify(shot.weapon),
        Float64Array.from(origin),
        Float64Array.from(target),
        Float64Array.from(velocity),
        scatter,
      ),
    );
    if (result.fired) paths.set(result.projectile, [origin]);
    return { label: shot.label, ...result };
  });
  const crossingShot = bench.shots.find((s) => s.label === "grenade across crossing bodies")!;
  const crossing = shots.find((s) => s.label === crossingShot.label)!;
  if (!crossing.fired || crossing.time_of_flight === undefined)
    throw new Error("the crossing demonstration needs a solved launch");
  if (!("ground" in crossingShot.aim)) throw new Error("the crossing aim must be ground");
  const [aimX, aimY] = crossingShot.aim.ground;
  const arrival = crossing.time_of_flight;
  const reach = aimX - crossingShot.from[0];
  // The tank reaches the aim on arrival. The soldier crosses the descending
  // segment one body diameter early, outside collision but inside near-miss reach.
  // Nominal flight time keeps the body script fixed when spread is toggled.
  const movers = bench.movers.map((m) => {
    if (m.id !== 4 && m.id !== 5) return m;
    const x = m.id === 5 ? aimX : m.start[0];
    const time = arrival * ((x - crossingShot.from[0]) / reach);
    const passed = m.id === 4 ? 2 * P.soldier_radius_m : 0;
    return { ...m, start: [x, aimY - m.velocity[1] * time + passed] as [number, number] };
  });
  return { lab, tick: 0, shots, events: [], paths, movers, bench };
}

/** What a struck label hit, as the battle publishes it. */
function hitKind(movers: readonly Mover[], struck: string | undefined): string {
  if (!struck?.startsWith("body")) return struck?.startsWith("prop") ? "prop" : "ground";
  const m = movers.find((b) => b.id === Number(struck.slice(5)));
  return m?.armored ? "hull" : m?.shape === "capsule" ? "soldier" : "prop";
}

/** Emitter `i`'s soldier id, as a publication would name the shooter. */
const EMITTER_BASE = 1000;

/** The publication of tick 0: every emitter seen, nothing fired yet. */
function launchPublication({ shots }: Bench): EffectPublication {
  return {
    tick: 0,
    segments: [],
    blasts: [],
    smokes: [],
    shooters: shots.map((s, i) => ({
      key: i,
      half: null,
      yaw: 0,
      position: [s.from[0], s.from[1], 0],
      members: [EMITTER_BASE + i],
      mounts: [{ bearing: 0, elevation: 0, shots: 0, kind: s.kind, muzzle: null }],
    })),
  };
}

/** Step the run one tick; what it drew, as a publication for the effects. */
function stepRun(run: Run, view: WorldView): EffectPublication {
  const k = run.tick + 1;
  run.lab.set_bodies(packBodies(view, run.movers, k));
  const out = JSON.parse(run.lab.step()) as {
    events: Omit<LabEvent, "tick">[];
    rounds: [number, number, number, number][];
  };
  run.tick = k;
  const from = new Map([...run.paths].map(([id, path]) => [id, path.length - 1]));
  const ended = new Map<number, Omit<LabEvent, "tick">>();
  const glances = new Map<number, { point: number; normal: Xyz }[]>();
  for (const e of out.events) {
    run.events.push({ tick: k, ...e });
    if (e.kind === "near_miss") continue;
    const path = run.paths.get(e.projectile);
    if (!path) continue;
    path.push(e.point);
    if (e.kind === "ricochet") {
      const list = glances.get(e.projectile) ?? [];
      list.push({ point: path.length - 1 - from.get(e.projectile)!, normal: e.normal! });
      glances.set(e.projectile, list);
    } else ended.set(e.projectile, e);
  }
  for (const [id, x, y, z] of out.rounds) run.paths.get(id)?.push([x, y, z]);
  const segments: EffectSegment[] = [];
  const blasts: EffectBlast[] = [];
  for (const [id, path] of run.paths) {
    const start = from.get(id)!;
    if (path.length - start < 2) continue;
    const shot = run.bench.shots[run.shots.findIndex((s) => s.projectile === id)];
    const end = ended.get(id);
    const hit = end?.kind === "impact" ? hitKind(run.movers, end.struck) : "none";
    segments.push({
      path: path.slice(start),
      ricochets: glances.get(id) ?? [],
      kind: shot.kind,
      shooter: EMITTER_BASE + run.bench.shots.indexOf(shot),
      hit,
      normal: hit === "none" ? null : (end?.normal ?? [0, 0, 1]),
    });
    const radius = (shot.weapon as { blast_radius_m?: number }).blast_radius_m ?? 0;
    if (end?.kind === "impact" && radius > 0)
      blasts.push({ point: end.point, radius, kind: shot.kind });
  }
  return {
    tick: k,
    segments,
    blasts,
    smokes: [],
    // Each emitter fired its one round at tick 0: counted from tick 1 on.
    shooters: run.bench.shots.map((s, i) => ({
      key: i,
      half: null,
      yaw: 0,
      position: [s.from[0], s.from[1], 0],
      members: [EMITTER_BASE + i],
      mounts: [
        {
          bearing: 0,
          elevation: 0,
          shots: run.shots[i].fired ? 1 : 0,
          kind: s.kind,
          muzzle: null,
        },
      ],
    })),
  };
}

function overlayOf(run: Run, view: WorldView, half: number) {
  const ended = new Map(run.events.filter(ends).map((e) => [e.projectile, e]));
  const traces: FlightTrace[] = [];
  const marks: FlightMark[] = [];
  for (const [id, points] of run.paths) {
    const end = ended.get(id);
    traces.push({
      points,
      outcome: !end
        ? "flying"
        : end.kind === "expired"
          ? "expired"
          : end.struck?.startsWith("body")
            ? "struck-body"
            : "struck-world",
    });
  }
  for (const shot of run.shots) {
    const { path, blocked_at: at } = shot;
    if (path && at) {
      // The rejected arc up to its obstruction. Horizontal reach grows
      // monotonically along any arc, so it orders the points.
      const reach = (p: readonly number[]) => Math.hypot(p[0] - path[0][0], p[1] - path[0][1]);
      traces.push({
        points: [...path.filter((p) => reach(p) < reach(at)), at],
        outcome: "blocked",
      });
      marks.push({ at, kind: "blocked" });
    }
  }
  for (const s of run.bench.shots) {
    if ("ground" in s.aim) {
      const [x, y] = s.aim.ground;
      marks.push({ at: [x, y, view.height_at(x, y) ?? 0], kind: "aim" });
    }
  }
  for (const e of run.events) {
    if (e.kind === "impact") {
      marks.push({
        at: e.point,
        normal: e.normal,
        kind: e.struck?.startsWith("body") ? "impact-body" : "impact-world",
      });
    } else if (e.kind === "ricochet") {
      marks.push({ at: e.point, normal: e.normal, kind: "ricochet" });
    } else if (e.kind === "near_miss") {
      marks.push({ at: e.point, kind: "near-miss" });
    }
  }
  const struck = new Set(
    run.events.filter((e) => e.struck?.startsWith("body")).map((e) => Number(e.struck!.slice(5))),
  );
  const bodies: FlightBody[] = run.movers.map((m) => {
    const p = poseAt(view, m, run.tick);
    return { shape: m.shape, base: p.base, yaw: p.yaw, dims: m.dims, struck: struck.has(m.id) };
  });
  return buildFlightOverlay(traces, marks, bodies, half);
}

export default function Ballistics() {
  return <SavedMap id={fixtureMap("ballistics")}>{(map) => <BallisticsLab map={map} />}</SavedMap>;
}

function BallisticsLab({ map }: { map: MapDefinition }) {
  const catalog = useSessionCatalog();
  const bench = useMemo(() => benchOf(catalog), [catalog]);
  const world = useStaticWorld(map, catalog.rules);
  const buildings = useStandingBuildings(world);
  const buildingsFeed = useFeed(buildings);
  const appearances = useMapAppearances(
    buildings?.placed ?? null,
    world?.exports.buildings.regionalFamily ?? null,
  );
  const meshes = useMemo(
    () =>
      world &&
      appearances &&
      buildWorldLayers(world.exports, world.layout, gameBiome, "surface", [], appearances),
    [world, appearances],
  );
  const worldFeed = useFeed(meshes);
  const [wasm, setWasm] = useState<Wasm | null>(null);
  const [spread, setSpread] = useState(false);
  const [playing, setPlaying] = useState(true);
  const [half, setHalf] = useState(0.3);
  // The run mutates in place as it steps; each step publishes a fresh wrapper
  // so presentation recomputes.
  const runRef = useRef<Run | null>(null);
  const [shown, setShown] = useState<{ run: Run } | null>(null);
  // Combat effects: each step noted as a publication, drawn at the clock of
  // the tick shown (the lab steps whole ticks, so its clock does too).
  const effects = useMemo(() => createEffectFrame(TICK_HZ), []);
  const effectBatch = useMemo(() => createEffectBatch(gameEffects.capacity), []);

  useEffect(() => {
    void loadWasm().then(setWasm);
  }, []);

  const restart = useCallback(
    (withSpread: boolean) => {
      if (!wasm || !world) return;
      runRef.current?.lab.free();
      runRef.current = startRun(wasm, bench, map, world.view, withSpread);
      effects.note(launchPublication(bench));
      setShown({ run: runRef.current });
    },
    [wasm, bench, map, world, effects],
  );
  useEffect(() => restart(false), [restart]);
  useEffect(() => () => runRef.current?.lab.free(), []);

  const runTo = useCallback(
    (tick: number) => {
      const run = runRef.current;
      if (!run || !world) return;
      while (run.tick < tick) effects.note(stepRun(run, world.view));
      setShown({ run });
    },
    [world, effects],
  );

  // Real-time playback: one tick per 1/30 s until every round has ended.
  useEffect(() => {
    if (!playing) return;
    const timer = setInterval(() => {
      const run = runRef.current;
      if (!run || !world) return;
      const live = run.paths.size - run.events.filter(ends).length;
      if (live === 0) return setPlaying(false);
      runTo(run.tick + 1);
    }, 1000 / TICK_HZ);
    return () => clearInterval(timer);
  }, [playing, world, runTo]);

  const overlay = useMemo(
    () => (shown && world ? overlayOf(shown.run, world.view, half) : undefined),
    [shown, world, half],
  );
  const overlayFeed = useFeed(overlay);

  const instances = useMemo<SceneInstance[]>(
    () =>
      world
        ? bench.shots.map((s) => ({
            kind: "infantry" as const,
            x: s.from[0],
            y: s.from[1],
            z: world.view.height_at(s.from[0], s.from[1]) ?? 0,
            yaw: 0,
            color: [0.9, 0.9, 0.85] as const,
          }))
        : [],
    [world, bench],
  );

  const diagnostics = useMemo(
    () => ({
      runTo,
      reset: (withSpread: boolean) => {
        setPlaying(false);
        setSpread(withSpread);
        restart(withSpread);
      },
      setLineHalfWidth: setHalf,
      state: () => {
        const run = runRef.current;
        return run && { tick: run.tick, shots: run.shots, events: run.events, movers: run.movers };
      },
      subsegments: () => runRef.current?.lab.subsegments_per_tick(),
      effects: () => effects.stats(),
    }),
    [runTo, restart, effects],
  );

  const frame = useCallback((): ViewportFrame | null => {
    const run = runRef.current;
    if (!run) return null;
    const clock = run.tick / TICK_HZ;
    return { clock, effects: effects.build(clock, effectBatch) };
  }, [effects, effectBatch]);

  const run = shown?.run;
  if (!world || !meshes || !run) return null;
  const impacts = run.events.filter((e) => e.kind === "impact");
  return (
    <>
      <LabViewport
        fixture="ballistics"
        world={worldFeed}
        buildings={buildingsFeed}
        appearances={appearances}
        overlay={overlayFeed}
        instances={instances}
        initialCamera={BALLISTICS_CAMERA}
        diagnostics={diagnostics}
        frame={frame}
      />
      <aside className="hud-panel lab-panel" data-testid="ballistics-panel">
        <strong>Ballistics</strong>
        <div className="lab-hint">
          Tick {run.tick} · {run.lab.subsegments_per_tick()} chord/tick · middle-drag orbit · WASD
          pan
        </div>
        <div className="lab-row">
          <button
            onClick={() => {
              restart(spread);
              setPlaying(true);
            }}
          >
            Fire salvo
          </button>
          <button aria-pressed={playing} onClick={() => setPlaying((p) => !p)}>
            {playing ? "Pause" : "Play"}
          </button>
          <button onClick={() => runTo(run.tick + 1)}>Step</button>
        </div>
        <label>
          <input
            type="checkbox"
            checked={spread}
            onChange={(e) => {
              setSpread(e.target.checked);
              restart(e.target.checked);
              setPlaying(true);
            }}
          />
          Weapon spread (off: analytic aim)
        </label>
        <ol className="lab-log lab-list" data-testid="shots">
          {run.shots.map((s) => {
            const end = run.events.find((e) => ends(e) && e.projectile === s.projectile);
            const glances = run.events.filter(
              (e) => e.kind === "ricochet" && e.projectile === s.projectile,
            ).length;
            return (
              <li key={s.label} className={s.fired ? undefined : "lab-rejected"}>
                {s.label}:{" "}
                {!s.fired
                  ? `no solution (${s.reason})`
                  : `${s.arc} arc → ${"↯ ".repeat(glances)}${end ? (end.struck ?? end.cause) : "in flight"}`}
              </li>
            );
          })}
        </ol>
        <div className="lab-hint">
          {impacts.length} impacts · {run.events.filter((e) => e.kind === "ricochet").length}{" "}
          ricochets · {run.events.filter((e) => e.kind === "near_miss").length} near misses
        </div>
        <div className="lab-hint">
          <div>
            Traces: orange hit a body · yellow hit ground · white in flight · faint red rejected arc
          </div>
          <div>
            Marks: red body hit · yellow ground hit · lime ricochet · black obstruction · cyan
            closest pass
          </div>
          <div>Navy plate: aim point · bodies blue, violet once struck</div>
        </div>
      </aside>
    </>
  );
}
