import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import {
  buildGuidanceOverlay,
  buildUnitMarks,
} from "@packages/battle-renderer/src/guidanceOverlay";
import { buildConsequenceOverlay } from "@packages/battle-renderer/src/consequenceOverlay";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { MountView, ObservationView } from "@web/battle/sim/observation";
import { REASON_TEXT } from "@web/battle/present/readouts";
import type { Order } from "@web/battle/sim/protocol";
import ambushMap from "@fixtures/ambush-lab.json";
import { AckLine } from "../AckLine";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario, type LabScript, type LabUnit } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { groundUnderRay, useStaticWorld } from "../useStaticWorld";

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

export const AMBUSH_CAMERA: Camera3DParams = {
  target: [300, 290, 0],
  distance: 640,
  pitch: 0.95,
  yaw: 3.14,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const LAUNCHER_MARK = [0.95, 0.95, 0.95, 1] as const;
const SCOUT_MARK = [0.55, 0.75, 1.0, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;

export default function Ambush() {
  const world = useStaticWorld(ambushMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  const [variant, setVariant] = useState<Variant>("prompt");
  const scenario = useMemo(
    () => labScenario(ambushMap, [...VARIANTS[variant].units], [], [...VARIANTS[variant].scripts]),
    [variant],
  );
  // Where each missile flew, for replaying the path to its last point.
  const trails = useRef(new Map<number, [number, number, number][]>());
  const missileNames = useRef(new Map<number, string>());
  // Where own rounds struck (kept 3 s) and how each missile ended.
  const impacts = useRef<{ at: [number, number, number]; tick: number }[]>([]);
  const [outcomes, setOutcomes] = useState<string[]>([]);
  const onDecoded = useCallback((o: ObservationView) => {
    impacts.current = impacts.current.filter((i) => o.tick - i.tick < 90);
    for (const p of o.projectiles)
      if (p.own && p.impact) impacts.current.push({ at: p.to, tick: o.tick });
    // A missile that has gone: report where it struck, as far as blue can tell.
    const flying = new Set(o.guided.map((g) => g.id));
    for (const [id, trail] of trails.current) {
      if (flying.has(id)) continue;
      const end = trail[trail.length - 1];
      const struck = impacts.current.find(
        (i) => Math.hypot(i.at[0] - end[0], i.at[1] - end[1]) < 12,
      );
      const name = missileNames.current.get(id) ?? "Missile";
      const line = struck ? `${name} struck at tick ${o.tick}` : `${name} ended without a strike`;
      setOutcomes((current) => [line, ...current].slice(0, 4));
      trails.current.delete(id);
    }
    for (const g of o.guided) {
      if (!missileNames.current.has(g.id))
        missileNames.current.set(g.id, `Missile ${missileNames.current.size + 1}`);
      const trail = trails.current.get(g.id) ?? [];
      trail.push(g.position);
      trails.current.set(g.id, trail.slice(-240));
    }
  }, []);
  const sim = useSimSession({ scenario, seed: SEED, onDecoded });
  const { observation } = sim;
  // A fresh battle starts with no missiles, marks or outcomes.
  useEffect(() => {
    trails.current.clear();
    missileNames.current.clear();
    impacts.current = [];
    setOutcomes([]);
  }, [sim.client]);
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
    const guidance = buildGuidanceOverlay(
      observation.guided.map((g) => ({ ...g, trail: trails.current.get(g.id) ?? [] })),
      surfaceZ,
    );
    const remains = buildConsequenceOverlay(
      observation.corpses,
      [],
      impacts.current.map((i) => ({ at: i.at, fade: 1 - (observation.tick - i.tick) / 90 })),
      surfaceZ,
    );
    // Name blue's launchers and scout on the map.
    const marks = buildUnitMarks(
      observation.own.map((u) => ({
        at: [u.position[0], u.position[1]] as const,
        color: u.kind === "at" ? LAUNCHER_MARK : SCOUT_MARK,
      })),
      surfaceZ,
    );
    const parts = [tracers, guidance, remains, marks];
    return {
      opaque: concatMeshes(parts.map((p) => p.opaque)),
      translucent: concatMeshes(parts.map((p) => p.translucent)),
    };
  }, [world, observation, surfaceZ]);

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
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    variant: (v: Variant) => setVariant(v),
    reset: () => sim.reset(),
    moveLauncher,
    stopLauncher,
    observeAs: (side: "blue" | "red") => sim.client?.observeAs(side),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
  };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="ambush"
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={AMBUSH_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="ambush-panel">
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

function describeLauncher(m: MountView): string {
  if (m.guiding) return "guiding a missile (the next one waits)";
  const reason = REASON_TEXT[m.reason] ?? m.reason;
  return m.reason === "no_own_sight" ? `${reason} (a scout's is not enough)` : reason;
}
