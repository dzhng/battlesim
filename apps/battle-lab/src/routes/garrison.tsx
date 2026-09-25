import { useCallback, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildStandingStructures, buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildEvidenceOverlay } from "@packages/battle-renderer/src/evidenceOverlay";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import {
  buildConsequenceOverlay,
  type ImpactMark,
} from "@packages/battle-renderer/src/consequenceOverlay";
import { buildGarrisonOverlay } from "@packages/battle-renderer/src/garrisonOverlay";
import { buildOrderOverlay } from "@packages/battle-renderer/src/orderOverlay";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import village from "@fixtures/village.json";
import garrisonMap from "@fixtures/garrison-lab.json";
import { AckLine } from "../AckLine";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { groundUnderRay, useStaticWorld, type StaticWorld } from "../useStaticWorld";

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
const IMPACT_TICKS = 60;
const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;
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

/** The static building under a camera ray, if the first thing it meets is one. */
function buildingUnderRay(world: StaticWorld, ray: LabPick["ray"]): number | null {
  const hit = world.view.raycast(...ray.origin, ...ray.dir, 10_000);
  if (!hit.length || hit[7] < 0) return null;
  const { props } = world.exports;
  const { propStride, propFields, propKinds } = world.layout;
  const [idAt, kindAt] = [propFields.indexOf("id"), propFields.indexOf("kind")];
  for (let r = 0; r * propStride < props.length; r++) {
    if (props[r * propStride + idAt] === hit[7]) {
      return propKinds[props[r * propStride + kindAt]] === "building" ? hit[7] : null;
    }
  }
  return null;
}

export default function Garrison() {
  const world = useStaticWorld(garrisonMap);
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

  // Buildings are drawn apart from the world: once blue has seen one fall,
  // it is no longer drawn and its known ruin stands in its place.
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface", "apart"),
    [world],
  );
  const fallenKey = (observation?.knownProps ?? [])
    .flatMap((p) => (p.replaces === null ? [] : [p.replaces]))
    .join();
  const standing = useMemo(
    () =>
      world &&
      buildStandingStructures(
        world.exports,
        world.layout,
        new Set(fallenKey ? fallenKey.split(",").map(Number) : []),
      ),
    [world, fallenKey],
  );

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
        .filter((u) => u.members.length > 0 && u.garrison?.phase !== "inside")
        .map((u) => ({ center: [u.position[0], u.position[1]], radius: 9, level: u.suppression })),
      marks,
      surfaceZ,
    );
    const garrisons = buildGarrisonOverlay(
      observation.own.flatMap((u) =>
        u.garrison
          ? [
              {
                center: [u.position[0], u.position[1]] as const,
                members: u.members,
                phase: u.garrison.phase,
                progress: u.garrison.progress,
                suppression: u.suppression,
              },
            ]
          : [],
      ),
      surfaceZ,
    );
    const orders = buildOrderOverlay(
      observation.own.filter((u) => control.selected.includes(u.id)),
      surfaceZ,
    );
    const parts = [evidence, tracers, remains, garrisons, orders];
    return {
      opaque: concatMeshes([standing!, ...parts.map((p) => p.opaque)]),
      translucent: concatMeshes(parts.map((p) => p.translucent)),
    };
  }, [world, observation, standing, surfaceZ, control.selected]);

  const onPick = useCallback(
    (pick: LabPick) => {
      const right = world && pick.button === "right";
      const ground = right ? groundUnderRay(world.view, pick.ray) : null;
      control.onPointer({
        ...pick,
        unit: pick.instance >= 0 ? (instanceUnits.current[pick.instance] ?? null) : null,
        ground: ground && [ground[0], ground[1]],
        building: right ? buildingUnderRay(world, pick.ray) : null,
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
    command: (order: Order, queued = false) => control.issue(order, queued),
    demo: (name: string) => runDemo(name),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
  };

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
        frameInstances={frameInstances}
        initialCamera={GARRISON_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
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
