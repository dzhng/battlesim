import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { GroundView } from "@web/battle/sim/ground";
import type { ObservationView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";
import game from "@fixtures/game.json";
import { BattleMemory, orderLayer, remainsLayer, tracerLayer } from "../battleOverlay";
import {
  buildGroundCellOverlay,
  CHANNEL_COLORS,
  groundCells,
  type GroundCells,
} from "../groundCells";
import { GROUND_CHANNELS, type GroundChannel } from "@web/battle/sim/ground";
import { channels } from "@web/battle/present/hudTheme";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { gameCamera } from "../gameCamera";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";

// The ground layer, as each side learns it. The lab field is the ground
// map's saved encounter (`fixtures/maps/ground/encounters/ground.json`): two
// tanks race east side by side, the north one through a crater field (two
// passes of HE bursts at tick 1, each thrown off its grid point as a barrage
// falls, so the field's craters are full); two
// tanks shell a red squad standing in craters and one in the open; a blue
// squad walks the field. `?street` inspects the street test map's `rear`
// encounter instead (`fixtures/maps/street/encounters/rear.json`): the street
// fight, with blue's supply truck and jeep pulled back behind the ridge and
// red's jeep driven in from beyond the east wood, where a shell lands beside
// it at the start, so each side learns ground, and a crater, the other never
// saw. The flat cell view
// draws the observed side's learned cells, rebuilt from the ground patches
// its publications carry; switching side reopens the stream with that side's
// full snapshot.

const SEED = 17;

const GROUND_CAMERA: Camera3DParams = {
  target: [300, 240, 0],
  distance: 470,
  pitch: 1.05,
  yaw: -1.57,
  ...gameCamera.lens,
};
/** Ticks between rebuilds of the cell view while the battle runs. */
const REFRESH_TICKS = 10;

/** The street inspector's camera: over the street, wide. */
const STREET_INSPECT_CAMERA: Camera3DParams = {
  ...gameCamera.opening(),
  target: [980, 800, 0],
  distance: 600,
};

export default function Ground() {
  const street = new URLSearchParams(window.location.search).has("street");
  return street ? <StreetGround /> : <LabFieldGround />;
}

function StreetGround() {
  return (
    <SavedEncounter fixture="street" encounter="rear">
      {(battle) => (
        <GroundInspector
          scenario={battle.scenario}
          seed={game.seed}
          camera={STREET_INSPECT_CAMERA}
          legend="the street test map's rear"
        />
      )}
    </SavedEncounter>
  );
}

function LabFieldGround() {
  return (
    <SavedEncounter fixture="ground" encounter="ground">
      {(battle) => <LabField battle={battle} />}
    </SavedEncounter>
  );
}

function LabField({ battle }: { battle: SavedBattle }) {
  // The race's line: where the encounter sends its first tank.
  const racer = battle.encounter.scripts[0].order;
  const raceGoalX = racer.kind === "move" ? racer.goal[0] : NaN;
  return (
    <GroundInspector
      scenario={battle.scenario}
      seed={SEED}
      camera={GROUND_CAMERA}
      legend={`the lab field (${game.ground.cell_m} m cells)`}
      extra={(observation) => {
        const own = observation?.own ?? [];
        const x = (id: number) => own.find((u) => u.id === id)?.position[0];
        const lag = (x(1) ?? 0) - (x(0) ?? 0);
        const corpses = observation?.corpses.filter((c) => !c.own) ?? [];
        return (
          <>
            <div data-testid="race">
              Race to x = {raceGoalX}: crater tank at {x(0)?.toFixed(0) ?? "—"} · clean tank at{" "}
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

interface InspectorProps {
  scenario: string;
  seed: number;
  camera: Camera3DParams;
  legend: string;
  extra?: (observation: ObservationView | null) => ReactNode;
}

function GroundInspector({ scenario, seed, camera, legend, extra }: InspectorProps) {
  const memory = useRef(new BattleMemory());
  const [side, setSide] = useState<SideName>("blue");
  const [cells, setCells] = useState<GroundCells | null>(null);
  const [shown, setShown] = useState<ReadonlySet<GroundChannel>>(new Set(GROUND_CHANNELS));
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
      if (!patch.full) maxDelta.current = Math.max(maxDelta.current, patchCells(patch));
      const view = groundRef.current?.current;
      if (!view) return;
      const b = built.current;
      if (
        view.epoch !== b.epoch ||
        (view.revision !== b.revision && o.tick - b.tick >= REFRESH_TICKS)
      )
        refreshGround(o.tick);
    },
    [refreshGround],
  );
  const session = useBattleSession({ scenario, seed, onDecoded, side });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation, client } = sim;
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
    const orders = orderLayer(
      session.units,
      observation,
      control.selected,
      session.revealed,
      surfaceZ,
    );
    const parts = [tracers, remains, orders];
    const view = cells ? buildGroundCellOverlay(cells, shown, surfaceZ) : new Float32Array();
    return combineWorldMeshes([{ translucent: view }, ...parts]);
  }, [
    world,
    observation,
    surfaceZ,
    control.selected,
    session.revealed,
    cells,
    shown,
    session.units,
  ]);
  const overlayFeed = useFeed(overlay);

  const toggle = (c: GroundChannel) =>
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
    show: (channels: GroundChannel[]) => setShown(new Set(channels)),
    observeAs,
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
              cells: patchCells(last),
            },
          }
        : null;
    },
  };

  if (!meshes) return null;
  const count = (c: GroundChannel) => cells?.cells.filter((cell) => cell.marks[c] > 0).length ?? 0;
  const last = observation?.groundPatch;
  return (
    <>
      <LabViewport
        fixture="ground"
        world={worldFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={camera}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="ground-panel">
        <strong>Ground layer</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <div className="lab-row">
          <button type="button" onClick={sim.restart}>
            Reset
          </button>
          {(["blue", "red"] as const).map((s) => (
            <button key={s} type="button" aria-pressed={side === s} onClick={() => observeAs(s)}>
              Learned by {s}
            </button>
          ))}
        </div>
        <div className="lab-row">
          {GROUND_CHANNELS.map((c) => (
            <button
              key={c}
              type="button"
              aria-pressed={shown.has(c)}
              onClick={() => toggle(c)}
              style={{ borderLeft: `6px solid rgb(${channels(CHANNEL_COLORS[c])})` }}
            >
              {c} ({count(c)})
            </button>
          ))}
        </div>
        <div data-testid="ground-stream">
          {last
            ? `Stream ${last.epoch} (${last.side}) at revision ${last.revision}; last patch ${
                last.full ? "a full snapshot" : `a delta from ${last.baseRevision}`
              } of ${patchCells(last)} cells; largest delta ${maxDelta.current}`
            : "No patch yet"}
        </div>
        <div className="lab-legend">
          Flat cells {side} has learned on {legend}: only ground its fog has shown, as it was when
          last seen. Stronger marks are more opaque; a cell shows its first shown channel.
          <br />
          Craters: {game.cover.crater} cover for infantry once {game.cover.crater_min_fill * 100}%
          full · vehicles ×{game.ground.crater_vehicle_mult} over a full crater. Scorch, tracks and
          trampling change nothing.
        </div>
        {extra?.(observation)}
      </aside>
    </>
  );
}

/** Logical cells represented by a compact patch; diagnostics never expand it. */
function patchCells(patch: ObservationView["groundPatch"]): number {
  let cells = 0;
  for (let at = 1; at < patch.runs.length; at += 4) cells += Math.floor(patch.runs[at] / 256);
  return cells;
}
