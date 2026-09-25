import { useCallback, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildEvidenceOverlay } from "@packages/battle-renderer/src/evidenceOverlay";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import {
  buildConsequenceOverlay,
  type ImpactMark,
} from "@packages/battle-renderer/src/consequenceOverlay";
import { buildOrderOverlay } from "@packages/battle-renderer/src/orderOverlay";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import village from "@fixtures/village.json";
import consequencesMap from "@fixtures/consequences-lab.json";
import { AckLine } from "../AckLine";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { groundUnderRay, useStaticWorld } from "../useStaticWorld";

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

export const CONSEQUENCES_CAMERA: Camera3DParams = {
  target: [330, 250, 0],
  distance: 330,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

const SOLDIER_HP = village.health.soldier;
const IMPACT_TICKS = 60;
const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;
const VEHICLE_HP: Record<string, number> = {
  tank: village.health.tank,
  supply: village.health.supply,
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
  const world = useStaticWorld(consequencesMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  // Where rounds struck recently, kept for two seconds so a hit can be read.
  const impacts = useRef<{ at: [number, number, number]; tick: number }[]>([]);
  const onDecoded = useCallback((o: ObservationView) => {
    impacts.current = impacts.current.filter((i) => o.tick - i.tick < IMPACT_TICKS);
    for (const p of o.projectiles) if (p.impact) impacts.current.push({ at: p.to, tick: o.tick });
  }, []);
  const sim = useSimSession({ scenario: SCENARIO, seed: SEED, onDecoded });
  const { observation } = sim;
  const control = useUnitControl(sim.client, observation);
  const instanceUnits = useRef<(number | null)[]>([]);
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;

  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );

  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      const drawn = sideInstances("blue", poses, observation, selectedRef.current);
      instanceUnits.current = drawn.owners;
      return drawn.instances;
    },
    [observation, sim.interpolator],
  );

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const evidence = buildEvidenceOverlay([], observation.knownProps, surfaceZ);
    const tracers = buildFlightOverlay(
      observation.projectiles.map((p) => ({
        points: [p.from, p.to],
        outcome: "flying" as const,
        color: p.own ? OWN_TRACER : ENEMY_TRACER,
      })),
      [],
      [],
      0.3,
    );
    const marks: ImpactMark[] = impacts.current.map((i) => ({
      at: i.at,
      fade: 1 - (observation.tick - i.tick) / IMPACT_TICKS,
    }));
    const remains = buildConsequenceOverlay(
      observation.corpses,
      observation.own
        .filter((u) => u.members.length > 0)
        .map((u) => ({ center: [u.position[0], u.position[1]], radius: 9, level: u.suppression })),
      marks,
      surfaceZ,
    );
    const orders = buildOrderOverlay(
      observation.own.filter((u) => control.selected.includes(u.id)),
      surfaceZ,
    );
    const parts = [evidence, tracers, remains, orders];
    return {
      opaque: concatMeshes(parts.map((p) => p.opaque)),
      translucent: concatMeshes(parts.map((p) => p.translucent)),
    };
  }, [world, observation, surfaceZ, control.selected]);

  const onPick = useCallback(
    (pick: LabPick) => {
      const ground = world && pick.button === "right" ? groundUnderRay(world.view, pick.ray) : null;
      control.onPointer({
        ...pick,
        unit: pick.instance >= 0 ? (instanceUnits.current[pick.instance] ?? null) : null,
        ground: ground && [ground[0], ground[1]],
      });
    },
    [world, control],
  );

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
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    acks: () => control.acks,
    demo: (name: string) => runDemo(name),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
  };

  if (!meshes) return null;
  const own = observation?.own ?? [];
  const fallen = observation?.corpses ?? [];
  return (
    <>
      <LabViewport
        fixture="consequences"
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={CONSEQUENCES_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
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
        <ul className="lab-log lab-list" data-testid="unit-health">
          {own.map((u) => (
            <li key={u.id}>
              <UnitHealth unit={u} />
            </li>
          ))}
        </ul>
        <div>
          Fallen: {fallen.filter((c) => c.own).length} blue · {fallen.filter((c) => !c.own).length}{" "}
          red seen
        </div>
        <div className="lab-hint">Commands, newest first</div>
        <ul className="lab-log" data-testid="ack-log">
          {control.acks.length === 0 && <li>None yet</li>}
          {control.acks.map((a) => (
            <AckLine key={a.seq} entry={a} />
          ))}
        </ul>
      </aside>
    </>
  );
}

function UnitHealth({ unit }: { unit: OwnUnitView }) {
  const vehicle = unit.members.length === 0;
  const health = vehicle
    ? unit.hp / VEHICLE_HP[unit.kind]
    : unit.memberHp.reduce((a, b) => a + b, 0) / (SOLDIER_HP * unit.memberHp.length || 1);
  return (
    <div className="lab-mount">
      <div>
        {unit.kind} #{unit.id} ·{" "}
        {vehicle ? `${unit.hp.toFixed(0)} hp` : `${unit.members.length} soldiers standing`}
      </div>
      <div className="lab-bar">
        <span>health</span>
        <meter min={0} max={1} low={0.35} high={0.7} optimum={1} value={health} />
        <span>{(health * 100).toFixed(0)}%</span>
      </div>
      {!vehicle && (
        <div className="lab-bar">
          <span>pinned</span>
          <meter min={0} max={1} low={0.3} high={0.6} optimum={0} value={unit.suppression} />
          <span>{(unit.suppression * 100).toFixed(0)}%</span>
        </div>
      )}
    </div>
  );
}
