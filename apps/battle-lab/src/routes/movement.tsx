import { useCallback, useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import { AckLog } from "../AckLog";
import { orderLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useBattleSession } from "../useBattleSession";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";

// A wall drops across the main road at tick 150: units only learn of it by
// coming close, then route around it.
const SEED = 4;

const MOVEMENT_CAMERA: Camera3DParams = {
  target: [120, 185, 0],
  distance: 390,
  pitch: 0.95,
  yaw: -1.5,
  ...gameCamera.lens,
};

/** Reference commands the lab can send, exactly as a player would. */
const DEMOS: Record<string, (own: OwnUnitView[]) => { order: Order; queued?: boolean }[]> = {
  "Shortest vs fastest": (own) => {
    const tanks = own.filter((u) => u.kind === "tank").map((u) => u.id);
    return [
      {
        order: {
          kind: "move",
          units: [tanks[0]],
          gesture: 9001,
          goal: [275, 185],
          route: "shortest",
        },
      },
      {
        order: {
          kind: "move",
          units: [tanks[1]],
          gesture: 9002,
          goal: [300, 215],
          route: "fastest",
        },
      },
    ];
  },
  "Group east": (own) => [
    {
      order: {
        kind: "move",
        units: own.map((u) => u.id),
        gesture: 9003,
        goal: [440, 150],
        route: "fastest",
      },
    },
  ],
  "Infantry through the gap": (own) => [
    {
      order: {
        kind: "move",
        units: own.filter((u) => u.kind === "rifle").map((u) => u.id),
        gesture: 9004,
        goal: [215, 325],
        route: "shortest",
      },
    },
  ],
  "Onto the cliff top": (own) => [
    {
      order: {
        kind: "move",
        units: [own.find((u) => u.kind === "supply")!.id],
        gesture: 9005,
        goal: [455, 345],
        route: "shortest",
      },
    },
  ],
};

export default function Movement() {
  return (
    <SavedEncounter map="movement" encounter="movement">
      {(battle) => <MovementLab battle={battle} />}
    </SavedEncounter>
  );
}

function MovementLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    return orderLayer(observation, control.selected, session.revealed, surfaceZ);
  }, [world, observation, control.selected, session.revealed, surfaceZ]);
  const overlayFeed = useFeed(overlay);
  // Obstacles blue has learned since setup (the tick-150 wall once met).

  const runDemo = useCallback(
    async (name: string) => {
      if (!observation) return;
      const steps = DEMOS[name](observation.own);
      control.setSelected([
        ...new Set(steps.flatMap((s) => ("units" in s.order ? s.order.units : []))),
      ]);
      for (const step of steps) await control.issue(step.order, step.queued ?? false);
    },
    [observation, control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, demo: (name: string) => runDemo(name) };

  if (!meshes) return null;
  const tick = observation?.tick ?? 0;
  return (
    <>
      <LabViewport
        fixture="movement"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={MOVEMENT_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="movement-panel">
        <strong>Movement</strong>
        <div className="lab-hint">
          Click/drag: select · Right‑click: move · Double right‑click: fast move · Shift: queue · S:
          stop · Arrows/screen edge: pan
        </div>
        <div>
          Tick {tick} · {sim.status.status}
          {tick < 150 ? ` · road wall appears at tick 150` : " · road wall is down"}
        </div>
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void runDemo(name)}>
              {name}
            </button>
          ))}
          <button type="button" onClick={control.stop} disabled={!control.selected.length}>
            Stop
          </button>
          <button type="button" onClick={sim.restart}>
            Reset
          </button>
        </div>
        <div className="lab-hint">
          Route colours: <span className="lab-shortest">shortest</span> ·{" "}
          <span className="lab-fastest">fastest</span> ·{" "}
          <span className="lab-rejected">blocked</span> ·{" "}
          <span className="lab-waiting">waiting</span>
        </div>
        <ul className="lab-log" data-testid="selection">
          {control.selectedUnits.length === 0 && <li>No unit selected</li>}
          {control.selectedUnits.map((u) => (
            <li key={u.id}>{describeUnit(u, control.unitName)}</li>
          ))}
        </ul>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}

function describeUnit(u: OwnUnitView, name: (id: number) => string): string {
  const who = `${u.kind} #${u.id}`;
  const queued = u.queue.length ? `, then ${u.queue.length} more` : "";
  switch (u.state) {
    case "route_blocked":
      return `${who}: route blocked — no known way to (${u.goal!.map((v) => v.toFixed(0)).join(", ")}); keeps the order`;
    case "waiting":
      return `${who}: waiting for ${u.blocker === null ? "traffic" : name(u.blocker)}${queued}`;
    case "moving":
      return `${who}: ${u.policy} route to (${u.goal!.map((v) => v.toFixed(0)).join(", ")})${queued}`;
    default:
      return `${who}: holding`;
  }
}
