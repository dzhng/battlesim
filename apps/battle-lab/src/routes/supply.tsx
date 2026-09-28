import { useCallback, useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { OwnUnitView } from "@web/battle/sim/observation";
import { serviceText } from "@web/battle/present/readouts";
import type { Order } from "@web/battle/sim/protocol";
import village from "@fixtures/village.json";
import supplyMap from "@fixtures/supply-lab.json";
import { AckLog } from "../AckLog";
import {
  deploymentLayer,
  orderLayer,
  remainsLayer,
  supplyLayer,
  tracerLayer,
} from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";
import { useFeed } from "../feed";

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

const SUPPLY_CAMERA: Camera3DParams = {
  target: [215, 215, 0],
  distance: 300,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

const SQUAD: Record<string, number> = {
  rifle: village.health.rifle_squad_size,
  recon: village.health.recon_squad_size,
  at: village.health.at_squad_size,
};
const HP: Record<string, number> = { tank: village.health.tank, supply: village.health.supply };
const WEAPONS = village.weapons as Record<string, { ammo: number | string }>;
const MOUNTS = village.mounts as Record<string, { weapons: string[] }[]>;

export default function Supply() {
  const session = useBattleSession({ map: supplyMap, scenario: SCENARIO, seed: SEED });
  const { world, meshes, rules, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const supply = supplyLayer(observation, rules.service.radius_m, surfaceZ);
    const setup = deploymentLayer(observation, surfaceZ);
    const orders = orderLayer(observation, control.selected, surfaceZ, control.showOrders);
    const tracers = tracerLayer(observation);
    const remains = remainsLayer(observation, null, surfaceZ, { suppression: false });
    const parts = [supply, orders, tracers, remains];
    return {
      opaque: concatMeshes([...parts.map((p) => p.opaque), setup]),
      translucent: concatMeshes(parts.map((p) => p.translucent)),
      animated: orders.animated,
      unoccluded: orders.unoccluded,
    };
  }, [world, observation, surfaceZ, control.selected, control.showOrders, rules]);
  const overlayFeed = useFeed(overlay);

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
  const diagnostics = { ...session.probes, demo: (name: string) => command(DEMOS[name]) };

  if (!meshes) return null;
  const own = observation?.own ?? [];
  return (
    <>
      <LabViewport
        fixture="supply"
        world={worldFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={SUPPLY_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
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
          <span className="lab-swatch lab-swatch-waiting" /> waiting for supply (broken ring; reason
          below)
          <br />
          green/orange ring on a truck: its set-up progress
        </div>
        <ul className="lab-log lab-list" data-testid="stock">
          {own
            .filter((u) => u.stock !== null)
            .map((u) => (
              <li key={u.id}>
                Truck #{u.id}: stock {u.stock} of {rules.service.stock} ·{" "}
                {u.stock === 0
                  ? "empty: serves nothing"
                  : setup(u, rules.service.deploy_and_pack_s)}
              </li>
            ))}
        </ul>
        <ul className="lab-log lab-list" data-testid="recipients">
          {own
            .filter((u) => u.stock === null)
            .map((u) => (
              <li key={u.id}>
                {describe(u)} — {serviceText(u)}
              </li>
            ))}
        </ul>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}

/** A truck's set-up state; setting up takes `setupS` seconds. */
function setup(u: OwnUnitView, setupS: number): string {
  const d = u.deployment;
  if (!d) return "";
  const s = (d.progress * setupS).toFixed(1);
  if (d.progress === 1) return "set up: serving its reach";
  return d.target === "deployed" ? `setting up ${s}/${setupS} s` : `packing up (${s} s set up)`;
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
