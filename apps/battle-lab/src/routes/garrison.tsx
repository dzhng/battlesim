import { useCallback, useEffect, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { ObservationView } from "@web/battle/sim/observation";
import { ReadoutLayer, SelectionCard } from "@web/battle/present/readouts";
import type { Order } from "@web/battle/sim/protocol";
import { AckLog } from "../AckLog";
import {
  BattleMemory,
  garrisonLayer,
  orderLayer,
  remainsLayer,
  tracerLayer,
} from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { durableSoldiers, VILLAGE_RULES } from "../scenarios";
import { useFeed } from "../feed";
import { villageCamera } from "../villageCamera";
import { TickStatus } from "../TickStatus";

// The garrison map's saved encounter
// (`fixtures/maps/garrison/encounters/garrison.json`). Blue's two rifle squads
// and a scout squad wait west of the building, out of
// red's sight. Red's squad stands east, behind the building, holding fire but
// spotting for red's tank 250 m east. The tank holds fire while blue's scouts
// walk in, enter and leave and a rifle squad takes the building after them
// (the scene's entry steps), then opens fire at the encounter's one scripted
// order and shells whatever occupants
// its squad sees. Held until then so its fire (the HMG wears walls)
// never brings the house down mid-entry: the entry steps measure the
// stationary timer, not a collapse. Blue holds fire until fired on. The
// building (prop 0) is 24 × 24 m and takes one squad of up to 16 soldiers.
const BUILDING = 0;
// Keep direct fire from eliminating the occupants before the house falls;
// collapse itself still decides which soldiers escape.
const RULES = durableSoldiers(VILLAGE_RULES);
const SEED = 11;

const GARRISON_CAMERA: Camera3DParams = {
  target: [346, 252, 0],
  distance: 95,
  pitch: 1.12,
  yaw: -1.57,
  ...villageCamera.lens,
};

/** Blue's squads, listed even once eliminated. */
const SQUADS = [
  { id: 0, kind: "rifle" },
  { id: 1, kind: "rifle" },
  { id: 2, kind: "recon" },
];

/** Reference commands, exactly as a player would send them. */
const DEMOS: Record<string, (o: ObservationView) => Order | null> = {
  "Scouts garrison": () => ({ kind: "garrison", units: [2], building: BUILDING }),
  "Rifle squad #0 garrisons": () => ({ kind: "garrison", units: [0], building: BUILDING }),
  "Scouts leave": () => ({ kind: "exit_building", units: [2] }),
  // Back west, out of red's sight behind the house: a squad in the open
  // behind it would be the tank's first target, one it holds fire on.
  "Scouts fall back west": () => ({
    kind: "move",
    units: [2],
    gesture: 1100,
    goal: [250, 250],
    route: "shortest",
  }),
  "Squad #1 crosses the ruin": () => ({
    kind: "move",
    units: [1],
    gesture: 1101,
    goal: [400, 250],
    route: "shortest",
  }),
};

export default function Garrison() {
  return (
    <SavedEncounter map="garrison" encounter="garrison" rules={RULES}>
      {(battle) => <GarrisonLab battle={battle} />}
    </SavedEncounter>
  );
}

function GarrisonLab({ battle }: { battle: SavedBattle }) {
  // Where rounds struck recently, kept for two seconds so a hit can be read.
  const memory = useRef(new BattleMemory());
  const onDecoded = useCallback((o: ObservationView) => memory.current.note(o), []);
  const session = useBattleSession({
    ...battle,
    seed: SEED,
    onDecoded,
    // Once blue has seen a building fall, it is no longer drawn and its known
    // ruin stands in its place.
    destroyable: "apart",
  });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  useEffect(() => memory.current.clear(), [sim.client]);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const tracers = tracerLayer(observation);
    const remains = remainsLayer(observation, memory.current, surfaceZ);
    const garrisons = garrisonLayer(observation, surfaceZ);
    const orders = orderLayer(observation, control.selected, session.revealed, surfaceZ);
    const parts = [tracers, remains, garrisons, orders];
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
  const diagnostics = {
    ...session.probes,
    demo: (name: string) => runDemo(name),
    /** The tick red's tank switches to fire at will: after every entry. */
    tankOpensFire: battle.encounter.scripts[0].tick,
  };

  if (!meshes) return null;
  const own = observation?.own ?? [];
  const garrisoned = control.selectedUnits.some((u) => u.garrison !== null);
  return (
    <>
      <LabViewport
        fixture="garrison"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        massing={session.massingFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={GARRISON_CAMERA}
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
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="garrison-panel">
        <strong>Garrisons and ruins</strong>
        <div className="lab-hint">
          Click: select · Right‑click a building: garrison (Shift queues) · Right‑click ground: move
        </div>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void runDemo(name)}>
              {name}
            </button>
          ))}
          <button type="button" onClick={control.exitBuilding} disabled={!garrisoned}>
            Leave building
          </button>
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
        </div>
        <div className="lab-legend">
          <span className="lab-swatch lab-swatch-occupant" /> occupant at a perimeter slot ·{" "}
          <span className="lab-swatch lab-swatch-ruin" /> ruin
          <br />
          <span className="lab-swatch lab-swatch-fallen-blue" /> blue fallen ·{" "}
          <span className="lab-swatch lab-swatch-impact" /> where a round struck (2 s)
          <br />
          <span className="lab-swatch lab-swatch-tracer-own" /> blue rounds ·{" "}
          <span className="lab-swatch lab-swatch-tracer-enemy" /> red rounds ·{" "}
          <span className="lab-swatch lab-swatch-route" /> selected squad's route
          <br />
          <span className="lab-swatch lab-swatch-unseen" /> ground blue cannot see
        </div>
        <SelectionCard units={own} own={own} rules={session.rules} />
        {SQUADS.filter(({ id }) => !own.some((u) => u.id === id)).map(({ id, kind }) => (
          <div key={id} className="lab-hint">
            {kind} #{id} · eliminated
          </div>
        ))}
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
