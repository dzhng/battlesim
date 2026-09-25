import { useCallback, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildOrderOverlay } from "@packages/battle-renderer/src/orderOverlay";
import { buildDeploymentOverlay } from "@packages/battle-renderer/src/deploymentOverlay";
import { buildSupplyOverlay } from "@packages/battle-renderer/src/supplyOverlay";
import { buildConsequenceOverlay } from "@packages/battle-renderer/src/consequenceOverlay";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import village from "@fixtures/village.json";
import supplyMap from "@fixtures/supply-lab.json";
import { AckLine } from "../AckLine";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { groundUnderRay, useStaticWorld } from "../useStaticWorld";

// A supply truck sets up among a damaged tank, an AT team short of missiles and
// a rifle squad with casualties. A second truck stands empty beside a scout
// team short of one soldier. (Service under incoming fire is pinned by the
// Rust tests, where a squad is shelled from beyond its reach.)
const SCENARIO = labScenario(supplyMap, [
  { side: "blue", kind: "supply", position: [200, 200] },
  {
    side: "blue",
    kind: "tank",
    position: [240, 175],
    engagement: "return_fire_only",
    condition: { hp: 40 },
  },
  {
    side: "blue",
    kind: "at",
    position: [245, 235],
    engagement: "return_fire_only",
    condition: { spent: { atgm: 3 } },
  },
  {
    side: "blue",
    kind: "rifle",
    position: [165, 245],
    engagement: "return_fire_only",
    condition: { casualties: 3 },
  },
  { side: "blue", kind: "supply", position: [330, 320], stock: 0 },
  // Beside the empty truck only: it waits for stock that never comes.
  {
    side: "blue",
    kind: "recon",
    position: [360, 330],
    engagement: "return_fire_only",
    condition: { casualties: 1 },
  },
]);
const SEED = 13;

export const SUPPLY_CAMERA: Camera3DParams = {
  target: [215, 215, 0],
  distance: 300,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

const RADIUS = village.service.radius_m;
const SETUP_S = village.service.deploy_and_pack_s;
const SQUAD: Record<string, number> = {
  rifle: village.health.rifle_squad_size,
  recon: village.health.recon_squad_size,
  at: village.health.at_squad_size,
};
const HP: Record<string, number> = { tank: village.health.tank, supply: village.health.supply };
const WEAPONS = village.weapons as Record<string, { ammo: number | string }>;
const MOUNTS = village.mounts as Record<string, { weapons: string[] }[]>;
const FULL_STOCK = village.service.stock;
const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;

const WAITING = new Set(["moving", "firing", "no_stock", "garrisoned", "source_not_deployed"]);
const REASON: Record<string, string> = {
  out_of_range: "no supply vehicle in reach",
  source_not_deployed: "supply vehicle not set up yet",
  moving: "waiting: must stand still",
  firing: "waiting: fired this moment",
  serving: "being served",
  no_stock: "waiting: the truck cannot pay for the next item",
  full: "nothing missing",
  garrisoned: "in a building: no replacements",
};

export default function Supply() {
  const world = useStaticWorld(supplyMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  const sim = useSimSession({ scenario: SCENARIO, seed: SEED });
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
    const trucks = observation.own.filter((u) => u.stock !== null);
    const supply = buildSupplyOverlay(
      // An empty truck reaches nobody: no ring.
      trucks
        .filter((u) => (u.stock ?? 0) > 0)
        .map((u) => ({
          center: [u.position[0], u.position[1]],
          radius: RADIUS,
          // Set up and standing.
          ready: u.deployment?.progress === 1 && u.state === "idle",
        })),
      observation.own
        .filter((u) => u.stock === null && (u.service === "serving" || WAITING.has(u.service)))
        .map((u) => ({
          center: [u.position[0], u.position[1]],
          state: u.service === "serving" ? ("serving" as const) : ("waiting" as const),
        })),
      surfaceZ,
    );
    const setup = buildDeploymentOverlay(
      trucks.flatMap((u) =>
        u.deployment ? [{ position: u.position, yaw: u.yaw, ...u.deployment }] : [],
      ),
      surfaceZ,
    );
    const orders = buildOrderOverlay(
      observation.own.filter((u) => control.selected.includes(u.id)),
      surfaceZ,
    );
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
    const remains = buildConsequenceOverlay(observation.corpses, [], [], surfaceZ);
    const parts = [supply, orders, tracers, remains];
    return {
      opaque: concatMeshes([...parts.map((p) => p.opaque), setup]),
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

  const command = useCallback(
    (order: Order) => {
      if ("units" in order) control.setSelected(order.units);
      return control.issue(order);
    },
    [control],
  );
  const DEMOS: Record<string, Order> = {
    "Relocate the truck": {
      kind: "move",
      units: [0],
      gesture: 9401,
      goal: [300, 260],
      route: "shortest",
    },
    "Tank: move off": {
      kind: "move",
      units: [1],
      gesture: 9402,
      goal: [240, 120],
      route: "shortest",
    },
  };

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    demo: (name: string) => command(DEMOS[name]),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
  };

  if (!meshes) return null;
  const own = observation?.own ?? [];
  return (
    <>
      <LabViewport
        fixture="supply"
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={SUPPLY_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="supply-panel">
        <strong>Supply</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void command(DEMOS[name])}>
              {name}
            </button>
          ))}
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
        </div>
        <div className="lab-legend">
          <span className="lab-swatch lab-swatch-supply-ready" /> reach of a set-up truck ·{" "}
          <span className="lab-swatch lab-swatch-supply-idle" /> reach, not set up
          <br />
          <span className="lab-swatch lab-swatch-serving" /> being served ·{" "}
          <span className="lab-swatch lab-swatch-waiting" /> waiting (broken ring; reason below)
          <br />
          green/orange ring on a truck: its set-up progress
        </div>
        <ul className="lab-log lab-list" data-testid="stock">
          {own
            .filter((u) => u.stock !== null)
            .map((u) => (
              <li key={u.id}>
                Truck #{u.id}: stock {u.stock} of {FULL_STOCK} ·{" "}
                {u.stock === 0 ? "empty: serves nothing" : setup(u)}
              </li>
            ))}
        </ul>
        <ul className="lab-log lab-list" data-testid="recipients">
          {own
            .filter((u) => u.stock === null)
            .map((u) => (
              <li key={u.id}>
                {describe(u)} — {REASON[u.service] ?? u.service}
              </li>
            ))}
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

function setup(u: OwnUnitView): string {
  const d = u.deployment;
  if (!d) return "";
  const s = (d.progress * SETUP_S).toFixed(1);
  if (d.progress === 1) return "set up: serving its reach";
  return d.target === "deployed" ? `setting up ${s}/${SETUP_S} s` : `packing up (${s} s set up)`;
}

function describe(u: OwnUnitView): string {
  const who = `${u.kind} #${u.id}`;
  if (u.members.length === 0) return `${who}: ${u.hp.toFixed(0)}/${HP[u.kind]} hp`;
  // Finite rounds only, named by weapon row (unlimited rifles are left out).
  const ammo = u.mounts
    .flatMap((m) =>
      m.ammo.flatMap((n, k) =>
        n === null
          ? []
          : [
              `${n}/${WEAPONS[MOUNTS[u.kind][m.mount].weapons[k]].ammo} ${MOUNTS[u.kind][m.mount].weapons[k].replace("_", " ")}`,
            ],
      ),
    )
    .join(", ");
  return `${who}: ${u.members.length}/${SQUAD[u.kind]} soldiers${ammo ? `, ${ammo}` : ""}`;
}
