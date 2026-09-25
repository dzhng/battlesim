import { useCallback, useEffect, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import village from "@fixtures/village.json";
import garrisonMap from "@fixtures/garrison-lab.json";
import { AckLog } from "../AckLog";
import {
  BattleMemory,
  evidenceLayer,
  garrisonLayer,
  orderLayer,
  remainsLayer,
  tracerLayer,
} from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";

// Blue's two rifle squads and a scout squad wait west of the building, out of
// red's sight. Red's squad stands east, behind the building, holding fire but
// spotting for red's tank 250 m east, which shells whatever occupants its
// squad sees. Blue holds fire until fired on. The building (prop 0) is
// 24 × 24 m and takes 16 soldiers.
const BUILDING = 0;
const SCENARIO = labScenario(garrisonMap, [
  { side: "blue", kind: "rifle", position: [290, 250], engagement: "return_fire_only" },
  { side: "blue", kind: "rifle", position: [268, 250], engagement: "return_fire_only" },
  { side: "blue", kind: "recon", position: [325, 250], engagement: "return_fire_only" },
  { side: "red", kind: "rifle", position: [475, 250], engagement: "return_fire_only" },
  { side: "red", kind: "tank", position: [620, 200], yaw: 2.9 },
]);
const SEED = 11;

export const GARRISON_CAMERA: Camera3DParams = {
  target: [346, 252, 0],
  distance: 95,
  pitch: 1.12,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

const SOLDIER_HP = village.health.soldier;
const SQUAD_SIZE: Record<string, number> = {
  rifle: village.health.rifle_squad_size,
  recon: village.health.recon_squad_size,
  at: village.health.at_squad_size,
};
/** Blue's squads, listed even once eliminated. */
const SQUADS = [
  { id: 0, kind: "rifle" },
  { id: 1, kind: "rifle" },
  { id: 2, kind: "recon" },
];
const PHASE_LABEL: Record<string, string> = {
  entering: "entering",
  waiting_for_room: "no room: waiting",
  inside: "inside",
  exiting: "leaving",
};

/** Reference commands, exactly as a player would send them. */
const DEMOS: Record<string, (o: ObservationView) => Order | null> = {
  "Garrison both rifle squads": () => ({ kind: "garrison", units: [0, 1], building: BUILDING }),
  "Scouts try to join": () => ({ kind: "garrison", units: [2], building: BUILDING }),
  "Rifle squad #1 leaves": () => ({ kind: "exit_building", units: [1] }),
  "Squad #1 crosses the ruin": () => ({
    kind: "move",
    units: [1],
    gesture: 1101,
    goal: [400, 250],
    route: "shortest",
  }),
};

export default function Garrison() {
  // Where rounds struck recently, kept for two seconds so a hit can be read.
  const memory = useRef(new BattleMemory());
  const onDecoded = useCallback((o: ObservationView) => memory.current.note(o), []);
  const session = useBattleSession({
    map: garrisonMap,
    scenario: SCENARIO,
    seed: SEED,
    onDecoded,
    // Once blue has seen a building fall, it is no longer drawn and its known
    // ruin stands in its place.
    buildings: "apart",
  });
  const { world, meshes, standing, sim, control, surfaceZ } = session;
  const { observation } = sim;
  useEffect(() => memory.current.clear(), [sim.client]);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const evidence = evidenceLayer(observation, surfaceZ, { contacts: false });
    const tracers = tracerLayer(observation);
    const remains = remainsLayer(observation, memory.current, surfaceZ);
    const garrisons = garrisonLayer(observation, surfaceZ);
    const orders = orderLayer(observation, control.selected, surfaceZ);
    const parts = [evidence, tracers, remains, garrisons, orders];
    return {
      opaque: concatMeshes([standing!, ...parts.map((p) => p.opaque)]),
      translucent: concatMeshes(parts.map((p) => p.translucent)),
    };
  }, [world, observation, standing, surfaceZ, control.selected]);

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
  const garrisoned = control.selectedUnits.some((u) => u.garrison !== null);
  return (
    <>
      <LabViewport
        fixture="garrison"
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={session.frameInstances}
        initialCamera={GARRISON_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="garrison-panel">
        <strong>Garrisons and ruins</strong>
        <div className="lab-hint">
          Click: select · Right‑click a building: garrison (Shift queues) · Right‑click ground: move
        </div>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
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
          <span className="lab-swatch lab-swatch-occupant" /> occupant at a perimeter slot (edge
          turns orange when pinned) · <span className="lab-swatch lab-swatch-ruin" /> ruin
          <br />
          <span className="lab-swatch lab-swatch-suppressed" /> pinned squad outside ·{" "}
          <span className="lab-swatch lab-swatch-fallen-blue" /> blue fallen ·{" "}
          <span className="lab-swatch lab-swatch-impact" /> where a round struck (2 s)
          <br />
          <span className="lab-swatch lab-swatch-tracer-own" /> blue rounds ·{" "}
          <span className="lab-swatch lab-swatch-tracer-enemy" /> red rounds ·{" "}
          <span className="lab-swatch lab-swatch-route" /> selected squad's route
          <br />
          <span className="lab-swatch lab-swatch-unseen" /> ground blue cannot see
        </div>
        <ul className="lab-log lab-list" data-testid="garrison-units">
          {SQUADS.map(({ id, kind }) => {
            const u = own.find((o) => o.id === id);
            return (
              <li key={id}>
                {u ? (
                  <SquadLine unit={u} />
                ) : (
                  <div className="lab-mount">
                    {kind} #{id} · eliminated
                  </div>
                )}
              </li>
            );
          })}
        </ul>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}

function SquadLine({ unit }: { unit: OwnUnitView }) {
  const g = unit.garrison;
  // Strength against the full squad, so losses show as well as wounds.
  const full = SQUAD_SIZE[unit.kind] * SOLDIER_HP;
  const strength = unit.memberHp.reduce((a, b) => a + b, 0) / full;
  const timer = g && (g.phase === "entering" || g.phase === "exiting");
  return (
    <div className="lab-mount">
      <div>
        {unit.kind} #{unit.id} · {unit.members.length} soldiers ·{" "}
        <span data-testid={`garrison-${unit.id}`}>
          {g
            ? `${PHASE_LABEL[g.phase]}${timer ? ` ${(g.progress * 100).toFixed(0)}%` : ""}`
            : "outside"}
        </span>
      </div>
      <div className="lab-bar">
        <span>strength</span>
        <meter min={0} max={1} low={0.35} high={0.7} optimum={1} value={strength} />
        <span>{(strength * 100).toFixed(0)}%</span>
      </div>
      <div className="lab-bar">
        <span>pinned</span>
        <meter min={0} max={1} low={0.3} high={0.6} optimum={0} value={unit.suppression} />
        <span>{(unit.suppression * 100).toFixed(0)}%</span>
      </div>
    </div>
  );
}
