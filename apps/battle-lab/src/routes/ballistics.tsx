import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import {
  buildFlightOverlay,
  type FlightBody,
  type FlightMark,
  type FlightTrace,
} from "@packages/battle-renderer/src/flightMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import geometryMap from "@fixtures/geometry-lab.json";
import village from "@fixtures/village.json";
import { LabViewport } from "../LabViewport";
import { useStaticWorld, type WorldView } from "../useStaticWorld";
import { loadWasm, type Wasm } from "@web/battle/sim/module";

// Flight reproduction bench. Scripted bodies move at constant velocity (one
// reverses after launch); emitters fire the village weapon rows through the
// same solve → spread → launch path weapons use, all at tick 0. Nothing here
// decides a hit: the Rust store reports every event, judging armoured bodies
// by the battle's hull policy, so failed penetrations may glance off.

const SEED = 20260925;
const TICK_HZ = village.tick_hz;
const W = village.weapons;
const P = village.physics;
// No village weapon has indirect-fire capability yet; this lab-only row
// exercises the opt-in high arc with a slow round whose apex fits the frame.
const LAB_MORTAR = {
  speed_mps: 45,
  scatter_mrad: 10,
  suppression_radius_m: 10,
  trajectory: "indirect",
};
// The oblique-AP preset: village AP pierces every face of the tank, so this
// lab-only row is a spent AP round (penetration below the front plate) whose
// failed penetrations may glance off.
const LAB_SPENT_AP = { ...W.tank_ap, penetration: 120 };

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
  from: [number, number];
  muzzle: number;
  aim: { body: number; height: number } | { ground: [number, number] };
}

const SOLDIER = [P.soldier_radius_m, P.soldier_height_m];
const PRESET_TANK: [number, number] = [238, 262];
const TANK = P.tank_half_extents_m;
const NORTH = Math.PI / 2;

const MOVERS: Mover[] = [
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
    start: [300, 138.8],
    yaw: NORTH,
    velocity: [0, 3],
  },
  {
    id: 5,
    unit: 5,
    shape: "box",
    dims: TANK,
    start: [372, 127.8],
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

const SHOTS: Shot[] = [
  ...[60, 120, 180].map(
    (range, i): Shot => ({
      label: `grenade arc ${range} m`,
      weapon: W.grenade,
      from: [20, 14 + 6 * i],
      muzzle: P.infantry_muzzle_m,
      aim: { ground: [20 + range, 14 + 6 * i] },
    }),
  ),
  {
    label: "hmg at sliding board",
    weapon: W.hmg,
    from: [20, 140],
    muzzle: 2,
    aim: { body: 1, height: 0.9 },
  },
  {
    label: "grenade at walker",
    weapon: W.grenade,
    from: [20, 96],
    muzzle: P.infantry_muzzle_m,
    aim: { body: 2, height: 0.85 },
  },
  {
    label: "grenade at dodger",
    weapon: W.grenade,
    from: [20, 112],
    muzzle: P.infantry_muzzle_m,
    aim: { body: 3, height: 0.85 },
  },
  {
    label: "direct grenade over crest",
    weapon: W.grenade,
    from: [100, 118],
    muzzle: P.infantry_muzzle_m,
    aim: { ground: [100, 292] },
  },
  {
    label: "indirect lab mortar over crest",
    weapon: LAB_MORTAR,
    from: [92, 118],
    muzzle: P.infantry_muzzle_m,
    aim: { ground: [92, 292] },
  },
  {
    label: "grenade across crossing bodies",
    weapon: W.grenade,
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
      from: [PRESET_TANK[0] + 16, PRESET_TANK[1] - 9 + 0.6 * k],
      muzzle: 2,
      aim: { body: 6, height: 0.5 + 0.45 * k },
    }),
  ),
];

const BALLISTICS_CAMERA: Camera3DParams = {
  target: [160, 150, 10],
  distance: 380,
  pitch: 0.62,
  yaw: -1.3,
  fovY: 0.8,
  aspect: 1,
  near: 1,
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
function packBodies(view: WorldView, k: number): Float64Array {
  const out: number[] = [];
  for (const m of MOVERS) {
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

function startRun(wasm: Wasm, view: WorldView, spread: boolean): Run {
  const lab = new wasm.FlightLab(
    JSON.stringify(geometryMap),
    JSON.stringify(village.physics),
    JSON.stringify(village.health.tank_armor),
    JSON.stringify(village.ricochet),
    TICK_HZ,
    SEED,
  );
  const ground = (xy: [number, number], lift: number): Xyz => [
    xy[0],
    xy[1],
    (view.height_at(xy[0], xy[1]) ?? 0) + lift,
  ];
  const paths = new Map<number, Xyz[]>();
  const shots = SHOTS.map((shot): ShotResult => {
    const origin = ground(shot.from, shot.muzzle);
    let target: Xyz;
    let velocity: Xyz = [0, 0, 0];
    if ("body" in shot.aim) {
      const { body, height } = shot.aim;
      const m = MOVERS.find((b) => b.id === body)!;
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
  return { lab, tick: 0, shots, events: [], paths };
}

function stepRun(run: Run, view: WorldView) {
  const k = run.tick + 1;
  run.lab.set_bodies(packBodies(view, k));
  const out = JSON.parse(run.lab.step()) as {
    events: Omit<LabEvent, "tick">[];
    rounds: [number, number, number, number][];
  };
  run.tick = k;
  for (const e of out.events) {
    run.events.push({ tick: k, ...e });
    if (e.kind !== "near_miss") run.paths.get(e.projectile)?.push(e.point);
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
  for (const s of SHOTS) {
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
  const bodies: FlightBody[] = MOVERS.map((m) => {
    const p = poseAt(view, m, run.tick);
    return { shape: m.shape, base: p.base, yaw: p.yaw, dims: m.dims, struck: struck.has(m.id) };
  });
  return buildFlightOverlay(traces, marks, bodies, half);
}

export default function Ballistics() {
  const world = useStaticWorld(geometryMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
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
      runRef.current = startRun(wasm, world.view, withSpread);
      setShown({ run: runRef.current });
    },
    [wasm, world],
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

  const instances = useMemo<SceneInstance[]>(
    () =>
      world
        ? SHOTS.map((s) => ({
            kind: "infantry" as const,
            x: s.from[0],
            y: s.from[1],
            z: world.view.height_at(s.from[0], s.from[1]) ?? 0,
            yaw: 0,
            color: [0.9, 0.9, 0.85] as const,
          }))
        : [],
    [world],
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
        return run && { tick: run.tick, shots: run.shots, events: run.events };
      },
      subsegments: () => runRef.current?.lab.subsegments_per_tick(),
    }),
    [runTo, restart],
  );

  const run = shown?.run;
  if (!world || !meshes || !run) return null;
  const impacts = run.events.filter((e) => e.kind === "impact");
  return (
    <>
      <LabViewport
        fixture="ballistics"
        world={meshes}
        overlay={overlay}
        instances={instances}
        initialCamera={BALLISTICS_CAMERA}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="ballistics-panel">
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
