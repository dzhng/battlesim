import { useCallback, useEffect, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { ObservationView } from "@web/battle/sim/observation";
import { SelectionPanel } from "@web/battle/present/readouts";
import type { Order } from "@web/battle/sim/protocol";
import consequencesMap from "@fixtures/consequences-lab.json";
import { AckLog } from "../AckLog";
import { BattleMemory, orderLayer, remainsLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";
import { useFeed } from "../feed";

// Everyone holds fire until a demo orders it, so each consequence is caused
// by one visible order. Red's squads stand in the open and 45 m inside a
// forest; blue's squad stands close to the open one; red's tank blocks the
// walled passage that blue's truck will want.
const SCENARIO = labScenario(consequencesMap, [
  { side: "blue", kind: "tank", position: [230, 250], engagement: "return_fire_only" },
  { side: "blue", kind: "rifle", position: [356, 166], engagement: "return_fire_only" },
  { side: "blue", kind: "supply", position: [200, 390] },
  { side: "red", kind: "rifle", position: [360, 148], engagement: "return_fire_only" },
  { side: "red", kind: "rifle", position: [375, 275], engagement: "return_fire_only" },
  {
    side: "red",
    kind: "tank",
    position: [520, 390],
    yaw: Math.PI / 2,
    engagement: "return_fire_only",
  },
]);
const SEED = 9;

const CONSEQUENCES_CAMERA: Camera3DParams = {
  target: [330, 250, 0],
  distance: 330,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
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
  // Where rounds struck recently, kept for two seconds so a hit can be read.
  const memory = useRef(new BattleMemory());
  const onDecoded = useCallback((o: ObservationView) => memory.current.note(o), []);
  const session = useBattleSession({
    map: consequencesMap,
    scenario: SCENARIO,
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
    const orders = orderLayer(observation, control.selected, surfaceZ, control.showOrders);
    const parts = [tracers, remains, orders];
    return combineWorldMeshes(parts);
  }, [world, observation, surfaceZ, control.selected, control.showOrders]);
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
        overlay={overlayFeed}
        fog={session.fogFeed}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={CONSEQUENCES_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="consequences-panel">
        <strong>Consequences of fire</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
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
          <span className="lab-swatch lab-swatch-suppressed" /> suppressed ·{" "}
          <span className="lab-swatch lab-swatch-impact" /> where a round struck (2 s)
          <br />
          <span className="lab-swatch lab-swatch-fallen-blue" /> blue fallen ·{" "}
          <span className="lab-swatch lab-swatch-fallen-red" /> red fallen ·{" "}
          <span className="lab-swatch lab-swatch-wreck" /> wreck
          <br />
          <span className="lab-swatch lab-swatch-tracer-own" /> blue rounds ·{" "}
          <span className="lab-swatch lab-swatch-tracer-enemy" /> red rounds
        </div>
        <SelectionPanel units={own} />
        <div>
          Fallen: {fallen.filter((c) => c.own).length} blue · {fallen.filter((c) => !c.own).length}{" "}
          red seen
        </div>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
