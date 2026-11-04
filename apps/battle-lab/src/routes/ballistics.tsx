import { shippedRules } from "@web/shippedRules";
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
import type { WeaponRow } from "@packages/scene-assets/src/units";
import { LabViewport } from "../LabViewport";
import type { ViewportFrame } from "../presentedFrame";
import { useStandingBuildings, useStaticWorld, type WorldView } from "../useStaticWorld";
import type { ResolvedRules, SessionCatalog } from "@web/battle/catalog/compose";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { gameBiome } from "../gameBiome";
import { loadWasm, type Wasm } from "@web/battle/sim/module";
import { useFeed } from "@web/feed";
import { gameCamera } from "../gameCamera";

// Flight reproduction bench. Scripted bodies move at constant velocity (one
// reverses after launch); emitters fire the game weapon rows through the
// same solve → spread → launch path weapons use, all at tick 0. Nothing here
// decides a hit: the Rust store reports every event, judging armoured bodies
// by the battle's hull policy, so failed penetrations may glance off.
// It has no battle, so no publication: its rounds draw as the overlay's
// traces and marks, not as the battle's combat effects.

const SEED = 20260925;
const TICK_HZ = shippedRules.tick_hz;
const P = shippedRules.physics;

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
  rules: ResolvedRules;
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
  // Use the shipped indirect row so the lab exercises the same trajectory
  // and spread metadata that an artillery unit carries in battle.
  const LAB_MORTAR = W.artillery_mortar;
  // The oblique-AP preset: game AP pierces every face of the tank, so this
  // lab-only row is a spent AP round (penetration below the front plate) whose
  // failed penetrations may glance off.
  const LAB_SPENT_AP = { ...W.tank_ap, penetration: 120 };

  const SOLDIER = [P.soldier_radius_m, P.soldier_height_m];
  const PRESET_TANK: [number, number] = [238, 262];
  const TANK = units.hull("test_tank")!.half_extents_m;
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
      label: "artillery mortar over crest",
      weapon: LAB_MORTAR,
      kind: "grenade",
      from: [92, 118],
      muzzle: P.infantry_muzzle_m,
      aim: { ground: [92, 300] },
    },
    {
      label: "artillery mortar inside minimum range",
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
  return { movers, shots, rules: catalog.rules, tankArmor: units.hull("test_tank")!.armor };
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

const artilleryRoute = window.location.pathname.endsWith("/artillery");

function diagramPoint([, y, z]: Xyz): [number, number] {
  // Side profile along the mortar's northward line: horizontal is range and
  // vertical is height, so the high arc reads as an arc instead of a top-down
  // line with no visible elevation.
  // Compress the real flight height so the long high arc fits beside the
  // ground profile; the labels retain the authored metre values.
  return [100 + (y / 300) * 800, 540 - z * 0.35];
}

function ArtilleryDiagram({ run, view }: { run: Run; view: WorldView }) {
  const mortar = run.shots.find((shot) => shot.label === "artillery mortar over crest");
  const path = mortar?.projectile === undefined ? [] : (run.paths.get(mortar.projectile) ?? []);
  const impact = run.events.find(
    (event) => event.projectile === mortar?.projectile && event.kind === "impact",
  );
  const line = path.map(diagramPoint);
  const ridge = Array.from({ length: 25 }, (_, i) => {
    const y = 130 + i * (160 / 24);
    const h = Math.sqrt(Math.max(0, 1 - ((y - 210) / 80) ** 2)) * 20;
    const z = view.height_at(100, y) ?? h;
    return [100 + (y / 300) * 800, 540 - z * 4];
  });
  const ridgeLine = [[100, 540], ...ridge, [900, 540]].map(([x, y]) => `${x},${y}`).join(" ");
  const flightLine = line.map(([x, y]) => `${x},${y}`).join(" ");
  const impactPoint = impact ? diagramPoint(impact.point) : null;
  return (
    <div className="artillery-lab-diagram" data-testid="artillery-diagram">
      <svg
        viewBox="0 0 1000 660"
        preserveAspectRatio="none"
        role="img"
        aria-label="Indirect artillery fire over a ridge"
      >
        <defs>
          <pattern id="artillery-grid" width="40" height="40" patternUnits="userSpaceOnUse">
            <path d="M 40 0 L 0 0 0 40" fill="none" stroke="rgba(132,169,150,.16)" />
          </pattern>
          <linearGradient id="artillery-ground" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#233b35" />
            <stop offset="1" stopColor="#101c1c" />
          </linearGradient>
          <filter id="artillery-glow" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="3" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>
        <rect x="0" y="0" width="1000" height="660" fill="url(#artillery-ground)" />
        <rect x="0" y="0" width="1000" height="660" fill="url(#artillery-grid)" />
        <path d="M100 540 H900" stroke="#789b83" strokeWidth="2" opacity=".65" />
        <polyline points={ridgeLine} fill="none" stroke="#c59049" strokeWidth="5" opacity=".9" />
        {flightLine && (
          <polyline
            points={flightLine}
            fill="none"
            stroke="#f2d47d"
            strokeWidth="4"
            strokeLinecap="round"
            strokeLinejoin="round"
            filter="url(#artillery-glow)"
          />
        )}
        {line[0] && <circle cx={line[0][0]} cy={line[0][1]} r="9" fill="#74d1e8" />}
        {impactPoint && (
          <>
            <circle
              cx={impactPoint[0]}
              cy={impactPoint[1]}
              r="22"
              fill="none"
              stroke="#f05b54"
              strokeWidth="3"
            />
            <circle cx={impactPoint[0]} cy={impactPoint[1]} r="6" fill="#f05b54" />
          </>
        )}
        <text x="430" y="48" className="artillery-diagram-kicker">
          INDIRECT FIRE / HIGH ARC
        </text>
        <text x="120" y="575" className="artillery-diagram-label">
          MORTAR POSITION
        </text>
        <text x="600" y="408" className="artillery-diagram-label">
          RIDGE CREST
        </text>
        <path d="M 660 416 L 660 458" stroke="#d6dfd2" strokeWidth="1" opacity=".55" />
        <text x="815" y="575" className="artillery-diagram-label">
          IMPACT / TERRAIN
        </text>
        <text x="120" y="605" className="artillery-diagram-note">
          145 M/S · 1,800 M MAX · 180 M MIN
        </text>
      </svg>
    </div>
  );
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
    const scatter = spread
      ? ((shot.weapon as { targets: { ground?: { scatter_mrad: number } } }).targets.ground
          ?.scatter_mrad ?? 0)
      : 0;
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

/** Step the run one tick. */
function stepRun(run: Run, view: WorldView) {
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
  const meshes = useMemo(
    () => world && buildWorldLayers(world.exports, world.layout, gameBiome, "surface"),
    [world],
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

  useEffect(() => {
    void loadWasm().then(setWasm);
  }, []);

  const restart = useCallback(
    (withSpread: boolean) => {
      if (!wasm || !world) return;
      runRef.current?.lab.free();
      runRef.current = startRun(wasm, bench, map, world.view, withSpread);
      setShown({ run: runRef.current });
    },
    [wasm, bench, map, world],
  );
  useEffect(() => restart(false), [restart]);
  useEffect(() => () => runRef.current?.lab.free(), []);

  const runTo = useCallback(
    (tick: number) => {
      const run = runRef.current;
      if (!run || !world) return;
      while (run.tick < tick) stepRun(run, world.view);
      setShown({ run });
    },
    [world],
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
    }),
    [runTo, restart],
  );

  const frame = useCallback((): ViewportFrame | null => {
    const run = runRef.current;
    if (!run) return null;
    return { clock: run.tick / TICK_HZ };
  }, []);

  const run = shown?.run;
  if (!world || !meshes || !run) return null;
  const impacts = run.events.filter((e) => e.kind === "impact");
  const panelShots = artilleryRoute
    ? run.shots.filter((shot) => shot.label.startsWith("artillery mortar"))
    : run.shots;
  return (
    <>
      {artilleryRoute && <ArtilleryDiagram run={run} view={world.view} />}
      <LabViewport
        fixture="ballistics"
        world={worldFeed}
        buildings={buildingsFeed}
        // This bench draws proxy emitters and flight marks; it does not need
        // the full roster appearance library. Leaving models uninstalled (no
        // `appearances`) keeps the diagnostic route within WebGPU's
        // texture-array limit on devices that cap arrays at 256 layers.
        overlay={overlayFeed}
        instances={instances}
        initialCamera={BALLISTICS_CAMERA}
        diagnostics={diagnostics}
        frame={frame}
      />
      <aside
        className={`hud-panel lab-panel${artilleryRoute ? " artillery-panel" : ""}`}
        data-testid="ballistics-panel"
      >
        <strong>{artilleryRoute ? "Artillery" : "Ballistics"}</strong>
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
          {panelShots.map((s) => {
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
        {artilleryRoute ? (
          <div className="lab-hint">Arc clears ridge · close fire held</div>
        ) : (
          <div className="lab-hint">
            <div>
              Traces: orange hit a body · yellow hit ground · white in flight · faint red rejected
              arc
            </div>
            <div>
              Marks: red body hit · yellow ground hit · lime ricochet · black obstruction · cyan
              closest pass
            </div>
            <div>Navy plate: aim point · bodies blue, violet once struck</div>
          </div>
        )}
      </aside>
    </>
  );
}
