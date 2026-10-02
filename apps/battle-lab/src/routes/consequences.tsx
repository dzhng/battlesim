import { useCallback, useEffect, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { ObservationView } from "@web/battle/sim/observation";
import { ReadoutLayer, SelectionCard } from "@web/battle/present/readouts";
import type { Order } from "@web/battle/sim/protocol";
import { AckLog } from "../AckLog";
import { BattleMemory, orderLayer, remainsLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// Everyone holds fire until a demo orders it, so each consequence is caused
// by one visible order. Red's squads stand in the open and 45 m inside a
// forest; blue's squad stands close to the open one; red's tank, already
// hit twice, blocks the walled passage that blue's truck will want. The wear
// settles the tank duel in blue's favour: one hit ends it, and blue fires
// three rounds before red's return fire could land its third. A lighter wear
// left the duel to the seed: at this range a round sometimes falls short, and
// two short rounds lost it.
const SEED = 9;

const CONSEQUENCES_CAMERA: Camera3DParams = {
  target: [330, 250, 0],
  distance: 330,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
};

/** Reference commands, exactly as a player would send them. */
const DEMOS: Record<string, (o: ObservationView) => Order | null> = {
  "HE on the open squad": () => ({
    kind: "attack",
    units: [0],
    target: { kind: "ground", point: [358, 152, 0] },
  }),
  "HE on the forest squad": () => ({
    kind: "attack",
    units: [0],
    target: { kind: "ground", point: [373, 273, 0] },
  }),
  "Destroy the red tank": (o) => {
    const tank = o.identified.find((e) => e.kind === "tank");
    return tank
      ? { kind: "attack", units: [0], target: { kind: "identified", id: tank.id } }
      : null;
  },
  "Truck east through the gap": () => ({
    kind: "move",
    units: [2],
    gesture: 9201,
    goal: [700, 390],
    route: "shortest",
  }),
};

export default function Consequences() {
  return (
    <SavedEncounter map="consequences" encounter="consequences">
      {(battle) => <ConsequencesLab battle={battle} />}
    </SavedEncounter>
  );
}

function ConsequencesLab({ battle }: { battle: SavedBattle }) {
  // Where rounds struck recently, kept for two seconds so a hit can be read.
  const memory = useRef(new BattleMemory());
  const onDecoded = useCallback((o: ObservationView) => memory.current.note(o), []);
  const session = useBattleSession({
    ...battle,
    seed: SEED,
    onDecoded,
  });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  useEffect(() => memory.current.clear(), [sim.client]);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const tracers = tracerLayer(observation);
    const remains = remainsLayer(observation, memory.current, surfaceZ);
    const orders = orderLayer(observation, control.selected, session.revealed, surfaceZ);
    const parts = [tracers, remains, orders];
    return combineWorldMeshes(parts);
  }, [world, observation, surfaceZ, control.selected, session.revealed]);
  const overlayFeed = useFeed(overlay);

  const runDemo = useCallback(
    async (name: string) => {
      const o = sim.latest.current;
      const order = o && DEMOS[name](o);
      if (!order) return null;
      if ("units" in order) control.setSelected(order.units);
      return control.issue(order);
    },
    [sim.latest, control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, demo: (name: string) => runDemo(name) };

  if (!meshes) return null;
  const own = observation?.own ?? [];
  const fallen = observation?.corpses ?? [];
  return (
    <>
      <LabViewport
        fixture="consequences"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        massing={session.massingFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={CONSEQUENCES_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={session.placePanels}
        diagnostics={diagnostics}
      />
      <ReadoutLayer
        own={observation?.own ?? []}
        rules={session.rules}
        selected={control.selected}
        handle={session.readouts}
      />
      <aside
        className="hud-panel lab-panel"
        data-occludes-readouts
        data-testid="consequences-panel"
      >
        <strong>Consequences of fire</strong>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void runDemo(name)}>
              {name}
            </button>
          ))}
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
        </div>
        <div className="lab-legend">
          <span className="lab-swatch lab-swatch-impact" /> where a round struck (2 s)
          <br />
          <span className="lab-swatch lab-swatch-fallen-blue" /> blue fallen ·{" "}
          <span className="lab-swatch lab-swatch-fallen-red" /> red fallen ·{" "}
          <span className="lab-swatch lab-swatch-wreck" /> wreck
          <br />
          <span className="lab-swatch lab-swatch-tracer-own" /> blue rounds ·{" "}
          <span className="lab-swatch lab-swatch-tracer-enemy" /> red rounds
        </div>
        <SelectionCard units={own} own={own} rules={session.rules} />
        <div>
          Fallen: {fallen.filter((c) => c.own).length} blue · {fallen.filter((c) => !c.own).length}{" "}
          red seen
        </div>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
