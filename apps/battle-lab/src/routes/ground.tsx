import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { ObservationView } from "@web/battle/sim/observation";
import village from "@fixtures/village.json";
import groundMap from "@fixtures/ground-lab.json";
import { BattleMemory, orderLayer, remainsLayer, tracerLayer } from "../battleOverlay";
import {
  buildGroundCellOverlay,
  CHANNEL_COLORS,
  CHANNELS,
  decodeGroundCells,
  type Channel,
  type GroundCells,
} from "../groundCells";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario, type LabEvent, type LabScript } from "../scenarios";

// The ground layer: two tanks race east side by side, the north one through
// an authored crater field; two tanks shell a red squad standing in craters
// and one in the open; a blue squad walks the field. The cell view shows the
// authoritative layer (craters, scorch, tracks, trampling) as flat cells.

/** HE bursts (the lab emitter) over a grid, at tick 1. */
function craters(x0: number, x1: number, y0: number, y1: number, step: number): LabEvent[] {
  const out: LabEvent[] = [];
  for (let x = x0; x <= x1; x += step)
    for (let y = y0; y <= y1; y += step)
      out.push({ tick: 1, burst: { point: [x, y], weapon: "tank_he" } });
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
/** Ticks between refreshes of the cell view while the battle runs. */
const REFRESH_TICKS = 10;

export default function Ground() {
  const memory = useRef(new BattleMemory());
  const onDecoded = useCallback((o: ObservationView) => memory.current.note(o), []);
  const session = useBattleSession({ map: groundMap, scenario: SCENARIO, seed: SEED, onDecoded });
  const { world, meshes, sim, control, surfaceZ } = session;
  const { observation, client } = sim;
  const [cells, setCells] = useState<GroundCells | null>(null);
  const [shown, setShown] = useState<ReadonlySet<Channel>>(new Set(CHANNELS));
  const fetched = useRef(-Infinity);

  const refreshGround = useCallback(async () => {
    if (!client) return null;
    const next = decodeGroundCells(await client.ground());
    setCells(next);
    return next;
  }, [client]);
  useEffect(() => {
    memory.current.clear();
    setCells(null);
    fetched.current = -Infinity;
  }, [client]);
  const tick = observation?.tick ?? 0;
  useEffect(() => {
    if (Math.abs(tick - fetched.current) < REFRESH_TICKS) return;
    fetched.current = tick;
    void refreshGround();
  }, [tick, refreshGround]);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const tracers = tracerLayer(observation);
    const remains = remainsLayer(observation, memory.current, surfaceZ);
    const orders = orderLayer(observation, control.selected, surfaceZ);
    const parts = [tracers, remains, orders];
    const view = cells ? buildGroundCellOverlay(cells, shown, surfaceZ) : new Float32Array();
    return {
      opaque: concatMeshes(parts.map((p) => p.opaque)),
      translucent: concatMeshes([view, ...parts.map((p) => p.translucent)]),
    };
  }, [world, observation, surfaceZ, control.selected, cells, shown]);

  const toggle = (c: Channel) =>
    setShown((s) => {
      const next = new Set(s);
      if (!next.delete(c)) next.add(c);
      return next;
    });

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    refreshGround,
    cells: () => cells,
    show: (channels: Channel[]) => setShown(new Set(channels)),
  };

  if (!meshes) return null;
  const own = observation?.own ?? [];
  const x = (id: number) => own.find((u) => u.id === id)?.position[0];
  const count = (c: Channel) => cells?.cells.filter((cell) => cell.marks[c] > 0).length ?? 0;
  const lag = (x(1) ?? 0) - (x(0) ?? 0);
  return (
    <>
      <LabViewport
        fixture="ground"
        world={meshes}
        overlay={overlay}
        fog={session.fog}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={GROUND_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="ground-panel">
        <strong>Ground layer</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <div className="lab-row">
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
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
        <div className="lab-legend">
          Flat cells of {village.ground.cell_m} m, the authoritative layer (a debug view, not blue's
          knowledge). Stronger marks are more opaque; a cell shows its first shown channel.
          <br />
          Craters: infantry cover {village.ground.crater_cover} (forest scale) · vehicles ×
          {village.ground.crater_vehicle_mult} over a full crater. Scorch, tracks and trampling
          change nothing.
        </div>
        <div data-testid="race">
          Race to x = {RACE_GOAL_X}: crater tank at {x(0)?.toFixed(0) ?? "—"} · clean tank at{" "}
          {x(1)?.toFixed(0) ?? "—"}
          {lag > 0.5 ? ` · craters cost ${lag.toFixed(1)} m` : ""}
        </div>
        <div>
          Red fallen seen:{" "}
          {observation?.corpses.filter((c) => !c.own && c.position[1] < 345).length ?? 0} in craters
          · {observation?.corpses.filter((c) => !c.own && c.position[1] >= 345).length ?? 0} in the
          open
        </div>
      </aside>
    </>
  );
}

function css([r, g, b]: readonly number[]) {
  return `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;
}
