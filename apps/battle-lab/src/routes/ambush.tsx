import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildUnitMarks } from "@packages/battle-renderer/src/guidanceOverlay";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { MountView, ObservationView } from "@web/battle/sim/observation";
import { REASON_TEXT } from "@web/battle/present/infoPanel";
import type { Order } from "@web/battle/sim/protocol";
import ambushMap from "@fixtures/ambush-lab.json";
import { AckLog } from "../AckLog";
import { BattleMemory, guidanceLayer, remainsLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario, type LabScript, type LabUnit } from "../scenarios";
import { useFeed } from "../feed";
import { villageCamera } from "../villageCamera";

// Red's first tank stands in the open beside a building it can duck behind;
// the second waits north, hidden from the AT team, seen only by blue's scout.
// The AT team launches at 3 s; the missile needs about 3 s to arrive. Each
// variant uses the same weapon data: only red's timing and blue's positions
// differ.
const RED: LabUnit[] = [
  {
    side: "red",
    kind: "tank",
    position: [600, 248],
    yaw: Math.PI / 2,
    engagement: "return_fire_only",
  },
  {
    side: "red",
    kind: "tank",
    position: [620, 60],
    yaw: -Math.PI / 2,
    engagement: "return_fire_only",
  },
];
const BLUE: LabUnit[] = [
  { side: "blue", kind: "at", position: [60, 340] },
  { side: "blue", kind: "recon", position: [350, 120], engagement: "return_fire_only" },
];
const CROSSFIRE: LabUnit = { side: "blue", kind: "at", position: [700, 470] };
const escape = (tick: number): LabScript => ({
  tick,
  side: "red",
  order: { kind: "move", units: [0], gesture: 1, goal: [600, 305], route: "shortest" },
});

const VARIANTS = {
  late: {
    label: "Late escape",
    describe: "Red's tank ducks behind the building beside it 5 s after the launch: too late.",
    units: [...RED, ...BLUE],
    scripts: [escape(240)],
  },
  prompt: {
    label: "Prompt escape",
    describe:
      "Red's tank ducks behind the building beside it the moment the missile launches: the launcher loses sight and the missile flies on to its last point.",
    units: [...RED, ...BLUE],
    scripts: [escape(92)],
  },
  crossfire: {
    label: "Prepared crossfire",
    describe:
      "The same prompt escape against two AT teams: the south-east team still sees the tank behind the building.",
    units: [...RED, ...BLUE, CROSSFIRE],
    scripts: [escape(92)],
  },
} as const;
type Variant = keyof typeof VARIANTS;
const SEED = 10;

const AMBUSH_CAMERA: Camera3DParams = {
  target: [300, 290, 0],
  distance: 640,
  pitch: 0.95,
  yaw: 3.14,
  ...villageCamera.lens,
};

const LAUNCHER_MARK = [0.95, 0.95, 0.95, 1] as const;
const SCOUT_MARK = [0.55, 0.75, 1.0, 1] as const;

export default function Ambush() {
  const [variant, setVariant] = useState<Variant>("prompt");
  const scenario = useMemo(
    () => labScenario(ambushMap, [...VARIANTS[variant].units], [], [...VARIANTS[variant].scripts]),
    [variant],
  );
  // Own strikes (kept 3 s) and each missile's path, for replaying it to its last point.
  const memory = useRef(new BattleMemory({ impactTicks: 90, ownImpactsOnly: true }));
  const missileNames = useRef(new Map<number, string>());
  // How each missile ended.
  const [outcomes, setOutcomes] = useState<string[]>([]);
  const onDecoded = useCallback((o: ObservationView) => {
    // The missiles gone this frame, and where each one's path ended.
    const flying = new Set(o.guided.map((g) => g.id));
    const ended = [...memory.current.trails].filter(([id]) => !flying.has(id));
    memory.current.note(o);
    // Report where each struck, as far as blue can tell.
    for (const [id, trail] of ended) {
      const end = trail[trail.length - 1];
      const struck = memory.current.impacts.find(
        (i) => Math.hypot(i.at[0] - end[0], i.at[1] - end[1]) < 12,
      );
      const name = missileNames.current.get(id) ?? "Missile";
      const line = struck ? `${name} struck at tick ${o.tick}` : `${name} ended without a strike`;
      setOutcomes((current) => [line, ...current].slice(0, 4));
    }
    for (const g of o.guided)
      if (!missileNames.current.has(g.id))
        missileNames.current.set(g.id, `Missile ${missileNames.current.size + 1}`);
  }, []);
  const session = useBattleSession({ map: ambushMap, scenario, seed: SEED, onDecoded });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  // A fresh battle starts with no missiles, marks or outcomes.
  useEffect(() => {
    memory.current.clear();
    missileNames.current.clear();
    setOutcomes([]);
  }, [sim.client]);
  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const tracers = tracerLayer(observation);
    const guidance = guidanceLayer(observation, memory.current, surfaceZ);
    const remains = remainsLayer(observation, memory.current, surfaceZ);
    // Name blue's launchers and scout on the map.
    const marks = buildUnitMarks(
      observation.own.map((u) => ({
        at: [u.position[0], u.position[1]] as const,
        color: u.kind === "at" ? LAUNCHER_MARK : SCOUT_MARK,
      })),
      surfaceZ,
    );
    const parts = [tracers, guidance, remains, marks];
    return combineWorldMeshes(parts);
  }, [world, observation, surfaceZ]);
  const overlayFeed = useFeed(overlay);

  const launchers = (observation?.own ?? []).filter((u) => u.kind === "at");
  const command = useCallback(
    (order: Order) => {
      if ("units" in order) control.setSelected(order.units);
      return control.issue(order);
    },
    [control],
  );
  const moveLauncher = () =>
    command({ kind: "move", units: [2], gesture: 9301, goal: [60, 400], route: "shortest" });
  const stopLauncher = () => command({ kind: "stop", units: [2] });

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    variant: (v: Variant) => setVariant(v),
    moveLauncher,
    stopLauncher,
    observeAs: (side: "blue" | "red") => sim.client?.observeAs(side),
  };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="ambush"
        world={worldFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={AMBUSH_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="ambush-panel">
        <strong>AT ambush</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <div className="lab-row" role="group" aria-label="Variant">
          {(Object.keys(VARIANTS) as Variant[]).map((v) => (
            <button
              key={v}
              type="button"
              aria-pressed={v === variant}
              onClick={() => setVariant(v)}
            >
              {VARIANTS[v].label}
            </button>
          ))}
        </div>
        <div className="lab-hint">{VARIANTS[variant].describe}</div>
        <div className="lab-row">
          <button type="button" onClick={() => void moveLauncher()}>
            West launcher: move
          </button>
          <button type="button" onClick={() => void stopLauncher()}>
            West launcher: Stop
          </button>
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
        </div>
        <div className="lab-legend">
          <span className="lab-swatch lab-swatch-guided" /> guided: steering at the target as its
          launcher sees it
          <br />
          <span className="lab-swatch lab-swatch-released" /> released: flying on to its fixed last
          point (✕)
          <br />
          <span className="lab-swatch lab-swatch-launcher" /> blue AT team ·{" "}
          <span className="lab-swatch lab-swatch-scout" /> blue scout ·{" "}
          <span className="lab-swatch lab-swatch-impact" /> where a round struck
          <br />
          <span className="lab-swatch lab-swatch-tracer-own" /> blue rounds ·{" "}
          <span className="lab-swatch lab-swatch-tracer-enemy" /> red rounds
        </div>
        <ul className="lab-log lab-list" data-testid="launchers">
          {launchers.map((u) => {
            const atgm = u.mounts.find((m) => m.guiding) ?? u.mounts[1];
            return (
              <li key={u.id}>
                AT team {u.position[0] < 400 ? "west" : "south-east"}: {describeLauncher(atgm)}
              </li>
            );
          })}
        </ul>
        <ul className="lab-log lab-list" data-testid="missiles">
          {(observation?.guided ?? []).length === 0 && <li>No missile in flight</li>}
          {outcomes.map((line, k) => (
            <li key={`o${k}`}>{line}</li>
          ))}
          {(observation?.guided ?? []).map((g) => (
            <li key={g.id}>
              {missileNames.current.get(g.id) ?? "Missile"}:{" "}
              {g.supported
                ? "guided by its launcher"
                : "released, flying on to its fixed last point (✕)"}
            </li>
          ))}
        </ul>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}

function describeLauncher(m: MountView): string {
  if (m.guiding) return "guiding a missile (the next one waits)";
  const reason = REASON_TEXT[m.reason] ?? m.reason;
  return m.reason === "no_own_sight" ? `${reason} (a scout's is not enough)` : reason;
}
