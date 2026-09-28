import { useCallback, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { OwnUnitView } from "@web/battle/sim/observation";
import { ReadoutLayer, serviceText, type ReadoutLayerHandle } from "@web/battle/present/readouts";
import type { Order } from "@web/battle/sim/protocol";
import supplyMap from "@fixtures/supply-lab.json";
import { UNITS, WEAPONS } from "@packages/scene-assets/src/shippedUnits";
import { AckLog } from "../AckLog";
import { orderLayer, supplyLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";
import { useFeed } from "../feed";
import { villageCamera } from "../villageCamera";

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
  ...villageCamera.lens,
};

export default function Supply() {
  const session = useBattleSession({ map: supplyMap, scenario: SCENARIO, seed: SEED });
  const { world, meshes, rules, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  const readouts = useRef<ReadoutLayerHandle>(null);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const supply = supplyLayer(
      observation,
      rules.service.radius_m,
      surfaceZ,
      control.selected,
      undefined,
      control.showOrders,
    );
    const orders = orderLayer(observation, control.selected, surfaceZ, control.showOrders);
    const tracers = tracerLayer(observation);
    return combineWorldMeshes([supply, orders, tracers]);
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
        onFrame={(project, camera) =>
          readouts.current?.place(
            project,
            camera.distance,
            session.panelAnchors(),
            session.drawnClock.current,
          )
        }
        diagnostics={diagnostics}
      />
      <ReadoutLayer own={own} rules={session.rules} selected={control.selected} handle={readouts} />
      <aside className="lab-panel" data-occludes-readouts data-testid="supply-panel">
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
          A selected truck&apos;s reach: <span className="lab-swatch lab-swatch-supply-ready" /> set
          up · <span className="lab-swatch lab-swatch-supply-idle" /> not set up
          <br />A unit in a set-up truck's reach says RESUPPLYING, SUPPLY FULL or CANNOT SUPPLY in its panel; why it waits, below
          <br />
          green/orange ring on a truck: its set-up progress
        </div>
        <ul className="lab-log lab-list" data-testid="stock">
          {own
            .filter((u) => u.stock !== null)
            .map((u) => (
              <li key={u.id}>
                Truck #{u.id}: stock {u.stock} of {capabilities(u).supply?.stock} ·{" "}
                {u.stock === 0
                  ? "empty: serves nothing"
                  : setup(u, capabilities(u).deploy?.seconds ?? 0)}
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

const capabilities = (u: OwnUnitView) => UNITS.type(u.kind).capabilities;

function describe(u: OwnUnitView): string {
  const who = `${u.kind} #${u.id}`;
  const hull = UNITS.hull(u.kind);
  if (hull) return `${who}: ${u.hp.toFixed(0)}/${hull.hp} hp`;
  const mounts = UNITS.type(u.kind).mounts;
  // Finite rounds only, named by weapon row (unlimited rifles are left out).
  const ammo = u.mounts
    .flatMap((m) =>
      m.ammo.flatMap((n, k) =>
        n === null
          ? []
          : [
              `${n}/${WEAPONS[mounts[m.mount].weapons[k]].ammo} ${mounts[m.mount].weapons[k].replace("_", " ")}`,
            ],
      ),
    )
    .join(", ");
  return `${who}: ${u.members.length}/${UNITS.slots(u.kind).length} soldiers${ammo ? `, ${ammo}` : ""}`;
}
