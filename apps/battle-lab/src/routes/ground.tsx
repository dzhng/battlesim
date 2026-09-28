import { mulberry32 } from "math/random";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { GroundView } from "@web/battle/sim/ground";
import type { ObservationView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";
import village from "@fixtures/village.json";
import groundMap from "@fixtures/ground-lab.json";
import { BattleMemory, orderLayer, remainsLayer, tracerLayer } from "../battleOverlay";
import {
  buildGroundCellOverlay,
  CHANNEL_COLORS,
  CHANNELS,
  groundCells,
  type Channel,
  type GroundCells,
} from "../groundCells";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { useBuiltScenario } from "../useBuiltScenario";
import { villageCamera } from "../villageCamera";
import { labScenario, type LabEvent, type LabScript } from "../scenarios";
import { useFeed } from "../feed";

// The ground layer, as each side learns it. The lab field: two tanks race
// east side by side, the north one through an authored crater field; two
// tanks shell a red squad standing in craters and one in the open; a blue
// squad walks the field. `?village` inspects the village encounter instead,
// paused after the supported attack's opening bombardment. The flat cell view
// draws the observed side's learned cells, rebuilt from the ground patches
// its publications carry; switching side reopens the stream with that side's
// full snapshot.

/** HE bursts (the lab emitter) at tick 1, one per `step` over the rect, each
 *  thrown up to 0.4 `step` off its grid point (seeded), as a barrage falls. */
function craters(x0: number, x1: number, y0: number, y1: number, step: number): LabEvent[] {
  const rng = mulberry32.create(Math.round(x0 * 1000 + y0));
  const off = () => (mulberry32.sample(rng) - 0.5) * 0.8 * step;
  const out: LabEvent[] = [];
  for (let x = x0; x <= x1; x += step)
    for (let y = y0; y <= y1; y += step) {
      const point: [number, number] = [
        Math.min(x1, Math.max(x0, x + off())),
        Math.min(y1, Math.max(y0, y + off())),
      ];
      out.push({ tick: 1, burst: { point, weapon: "tank_he" } });
    }
  return out;
}

/** The crater field the north tank crosses. */
const CRATER_FIELD = { x0: 220, x1: 380, y0: 96, y1: 124 };
const RACE_GOAL_X = 500;
const BARRAGE: [number, number][] = [
  [470, 310],
  [470, 380],
];

const move = (unit: number, goal: [number, number]): LabScript => ({
  tick: 1,
  side: "blue",
  order: { kind: "move", units: [unit], gesture: 9700 + unit, goal, route: "shortest" },
});

const SCENARIO = labScenario(
  groundMap,
  [
    { side: "blue", kind: "tank", position: [110, 110], engagement: "return_fire_only" },
    { side: "blue", kind: "tank", position: [110, 170], engagement: "return_fire_only" },
    { side: "blue", kind: "tank", position: [120, 300], engagement: "return_fire_only" },
    { side: "blue", kind: "tank", position: [120, 390], engagement: "return_fire_only" },
    { side: "blue", kind: "rifle", position: [140, 235], engagement: "return_fire_only" },
    {
      side: "red",
      kind: "rifle",
      position: BARRAGE[0],
      yaw: Math.PI,
      engagement: "return_fire_only",
    },
    {
      side: "red",
      kind: "rifle",
      position: BARRAGE[1],
      yaw: Math.PI,
      engagement: "return_fire_only",
    },
  ],
  [
    // Two passes, so the field's craters are full.
    ...craters(CRATER_FIELD.x0, CRATER_FIELD.x1, CRATER_FIELD.y0, CRATER_FIELD.y1, 4),
    ...craters(CRATER_FIELD.x0 + 2, CRATER_FIELD.x1, CRATER_FIELD.y0 + 2, CRATER_FIELD.y1, 4),
    // The red squad's own foxhole craters.
    ...craters(455, 485, 298, 322, 6),
  ],
  [
    move(0, [RACE_GOAL_X, 110]),
    move(1, [RACE_GOAL_X, 170]),
    move(4, [460, 235]),
    ...BARRAGE.map(
      (point, k): LabScript => ({
        tick: 1,
        side: "blue",
        order: {
          kind: "attack",
          units: [2 + k],
          target: { kind: "ground", point: [point[0], point[1], 0] },
        },
      }),
    ),
  ],
);
const SEED = 17;

const GROUND_CAMERA: Camera3DParams = {
  target: [300, 240, 0],
  distance: 470,
  pitch: 1.05,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};
/** The village inspector: the supported attack two minutes in, paused. */
const VILLAGE_SCRIPT = "scout-suppress-flank";
const VILLAGE_WARM_TICKS = 120 * village.tick_hz;
const VILLAGE_INSPECT_CAMERA: Camera3DParams = { ...villageCamera.opening(), distance: 900 };
/** Ticks between rebuilds of the cell view while the battle runs. */
const REFRESH_TICKS = 10;

export default function Ground() {
  const inspectVillage = new URLSearchParams(window.location.search).has("village");
  return inspectVillage ? <VillageGround /> : <LabFieldGround />;
}

function LabFieldGround() {
  return (
    <GroundInspector
      map={groundMap}
      scenario={SCENARIO}
      seed={SEED}
      camera={GROUND_CAMERA}
      legend={`the lab field (${village.ground.cell_m} m cells)`}
      extra={(observation) => {
        const own = observation?.own ?? [];
        const x = (id: number) => own.find((u) => u.id === id)?.position[0];
        const lag = (x(1) ?? 0) - (x(0) ?? 0);
        const corpses = observation?.corpses.filter((c) => !c.own) ?? [];
        return (
          <>
            <div data-testid="race">
              Race to x = {RACE_GOAL_X}: crater tank at {x(0)?.toFixed(0) ?? "—"} · clean tank at{" "}
              {x(1)?.toFixed(0) ?? "—"}
              {lag > 0.5 ? ` · craters cost ${lag.toFixed(1)} m` : ""}
            </div>
            <div>
              Red fallen seen: {corpses.filter((c) => c.position[1] < 345).length} in craters ·{" "}
              {corpses.filter((c) => c.position[1] >= 345).length} in the open
            </div>
          </>
        );
      }}
    />
  );
}

function VillageGround() {
  const built = useBuiltScenario("ordinary", (wasm, v) =>
    wasm.village_scenario(JSON.stringify(village), v),
  );
  if (!built) return null;
  if (typeof built !== "string")
    return (
      <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
        the village scenario could not be built: {built.error}
      </main>
    );
  return (
    <GroundInspector
      map={village.map}
      scenario={built}
      seed={village.seed}
      camera={VILLAGE_INSPECT_CAMERA}
      script={VILLAGE_SCRIPT}
      legend={`the village, ${VILLAGE_SCRIPT} from tick ${VILLAGE_WARM_TICKS}, paused`}
    />
  );
}

interface InspectorProps {
  map: unknown;
  scenario: string;
  seed: number;
  camera: Camera3DParams;
  legend: string;
  /** Blue is played by this comparison script, warmed and then paused. */
  script?: string;
  extra?: (observation: ObservationView | null) => ReactNode;
}

function GroundInspector({ map, scenario, seed, camera, legend, script, extra }: InspectorProps) {
  const memory = useRef(new BattleMemory());
  const [side, setSide] = useState<SideName>("blue");
  const [warm, setWarm] = useState(!script);
  const warmRef = useRef(warm);
  warmRef.current = warm;
  const [cells, setCells] = useState<GroundCells | null>(null);
  const [shown, setShown] = useState<ReadonlySet<Channel>>(new Set(CHANNELS));
  // The cell view was built from this tick and stream revision.
  const built = useRef({ tick: -Infinity, epoch: -1, revision: -1 });
  // The largest delta so far: the stream stays bounded.
  const maxDelta = useRef(0);
  // The session's view of the side's learned ground (set once it exists).
  const groundRef = useRef<{ current: GroundView | null } | null>(null);
  const refreshGround = useCallback((tick?: number) => {
    const view = groundRef.current?.current;
    if (!view) return null;
    const next = groundCells(view);
    built.current = {
      tick: tick ?? built.current.tick,
      epoch: view.epoch,
      revision: view.revision,
    };
    setCells(next);
    return next;
  }, []);
  // Every decoded frame, before its credit returns: the view already holds
  // its patch. A new stream redraws at once; learning within one at a pace.
  const onDecoded = useCallback(
    (o: ObservationView) => {
      memory.current.note(o);
      const patch = o.groundPatch;
      if (!patch.full) maxDelta.current = Math.max(maxDelta.current, patch.cells.length);
      const view = groundRef.current?.current;
      if (!view || !warmRef.current) return;
      const b = built.current;
      if (
        view.epoch !== b.epoch ||
        (view.revision !== b.revision && o.tick - b.tick >= REFRESH_TICKS)
      )
        refreshGround(o.tick);
    },
    [refreshGround],
  );
  const clientRef = useRef<{ pause(): void } | null>(null);
  const scripted = useMemo(
    () =>
      script
        ? {
            script,
            warmTo: VILLAGE_WARM_TICKS,
            onWarm: () => {
              clientRef.current?.pause();
              warmRef.current = true;
              setWarm(true);
              refreshGround();
            },
            onTick: () => {},
          }
        : undefined,
    [script, refreshGround],
  );
  const session = useBattleSession({ map, scenario, seed, onDecoded, scripted, side });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation, client } = sim;
  clientRef.current = client;
  groundRef.current = sim.ground;
  useEffect(() => {
    memory.current.clear();
    setCells(null);
    maxDelta.current = 0;
    built.current = { tick: -Infinity, epoch: -1, revision: -1 };
  }, [client]);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const tracers = tracerLayer(observation);
    const remains = remainsLayer(observation, memory.current, surfaceZ);
    const orders = orderLayer(observation, control.selected, surfaceZ, control.showOrders);
    const parts = [tracers, remains, orders];
    const view = cells ? buildGroundCellOverlay(cells, shown, surfaceZ) : new Float32Array();
    return combineWorldMeshes([{ translucent: view }, ...parts]);
  }, [world, observation, surfaceZ, control.selected, control.showOrders, cells, shown]);
  const overlayFeed = useFeed(overlay);

  const toggle = (c: Channel) =>
    setShown((s) => {
      const next = new Set(s);
      if (!next.delete(c)) next.add(c);
      return next;
    });
  const observeAs = (next: SideName) => {
    setSide(next);
    client?.observeAs(next);
  };

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    refreshGround,
    cells: () => cells,
    show: (channels: Channel[]) => setShown(new Set(channels)),
    observeAs,
    warm: () => warm,
    /** Start measuring the largest delta afresh. */
    resetLargestDelta: () => (maxDelta.current = 0),
    /** The view's stream and the latest patch, as the side received them. */
    ground: () => {
      const view = sim.ground.current;
      const last = sim.latest.current?.groundPatch;
      return view && last
        ? {
            epoch: view.epoch,
            side: view.side,
            revision: view.revision,
            largestDelta: maxDelta.current,
            patch: {
              epoch: last.epoch,
              side: last.side,
              full: last.full,
              base: last.baseRevision,
              revision: last.revision,
              cells: last.cells.length,
            },
          }
        : null;
    },
  };

  if (!meshes) return null;
  const count = (c: Channel) => cells?.cells.filter((cell) => cell.marks[c] > 0).length ?? 0;
  const last = observation?.groundPatch;
  return (
    <>
      <LabViewport
        fixture="ground"
        world={worldFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={camera}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="ground-panel">
        <strong>Ground layer</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {warm ? sim.status.status : "warming up"}
        </div>
        <div className="lab-row">
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
          {(["blue", "red"] as const).map((s) => (
            <button key={s} type="button" aria-pressed={side === s} onClick={() => observeAs(s)}>
              Learned by {s}
            </button>
          ))}
        </div>
        <div className="lab-row">
          {CHANNELS.map((c) => (
            <button
              key={c}
              type="button"
              aria-pressed={shown.has(c)}
              onClick={() => toggle(c)}
              style={{ borderLeft: `6px solid ${css(CHANNEL_COLORS[c])}` }}
            >
              {c} ({count(c)})
            </button>
          ))}
        </div>
        <div data-testid="ground-stream">
          {last
            ? `Stream ${last.epoch} (${last.side}) at revision ${last.revision}; last patch ${
                last.full ? "a full snapshot" : `a delta from ${last.baseRevision}`
              } of ${last.cells.length} cells; largest delta ${maxDelta.current}`
            : "No patch yet"}
        </div>
        <div className="lab-legend">
          Flat cells {side} has learned on {legend}: only ground its fog has shown, as it was when
          last seen. Stronger marks are more opaque; a cell shows its first shown channel.
          <br />
          Craters: {village.cover.crater} cover for infantry once{" "}
          {village.cover.crater_min_fill * 100}% full · vehicles ×
          {village.ground.crater_vehicle_mult} over a full crater. Scorch, tracks and trampling
          change nothing.
        </div>
        {extra?.(observation)}
      </aside>
    </>
  );
}

function css([r, g, b]: readonly number[]) {
  return `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;
}
