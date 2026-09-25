import { useCallback, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildOrderOverlay } from "@packages/battle-renderer/src/orderOverlay";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import {
  CommandBar,
  ReadoutLayer,
  SelectionPanel,
  type ReadoutLayerHandle,
} from "@web/battle/present/readouts";
import readoutsMap from "@fixtures/readouts-lab.json";
import { AckLine } from "../AckLine";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { buildingUnderRay, groundUnderRay, useStaticWorld } from "../useStaticWorld";

// A tank (cannon with AP/HE, and an HMG), an AT team, a rifle squad and a
// supply truck setting up, facing a red tank: every kind of timer runs at once.
const SCENARIO = labScenario(readoutsMap, [
  { side: "blue", kind: "tank", position: [200, 220] },
  { side: "blue", kind: "at", position: [215, 270] },
  { side: "blue", kind: "rifle", position: [185, 170], condition: { spent: { grenade: 3 } } },
  { side: "blue", kind: "supply", position: [140, 230] },
  { side: "red", kind: "tank", position: [470, 230], yaw: Math.PI, engagement: "return_fire_only" },
]);
const SEED = 14;

export const READOUTS_CAMERA: Camera3DParams = {
  target: [215, 230, 0],
  distance: 400,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;

export default function Readouts() {
  const world = useStaticWorld(readoutsMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  const sim = useSimSession({ scenario: SCENARIO, seed: SEED });
  const { observation } = sim;
  const control = useUnitControl(sim.client, observation);
  const drawn = useRef<{ owners: (number | null)[]; enemies: (number | null)[] }>({
    owners: [],
    enemies: [],
  });
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;
  const readouts = useRef<ReadoutLayerHandle>(null);
  // The drawn (interpolated) position of each own unit, for anchoring rings.
  const drawnAt = useRef(new Map<number, readonly [number, number, number]>());

  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );

  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      const d = sideInstances("blue", poses, observation, selectedRef.current);
      drawnAt.current = new Map(poses.map((p) => [p.id, p.position]));
      drawn.current = d;
      return d.instances;
    },
    [observation, sim.interpolator],
  );

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
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
    return {
      opaque: concatMeshes([orders.opaque, tracers.opaque]),
      translucent: concatMeshes([orders.translucent, tracers.translucent]),
    };
  }, [world, observation, surfaceZ, control.selected]);

  const onPick = useCallback(
    (pick: LabPick) => {
      const ground = world && pick.button === "right" ? groundUnderRay(world.view, pick.ray) : null;
      const k = pick.instance;
      control.onPointer({
        ...pick,
        unit: k >= 0 ? (drawn.current.owners[k] ?? null) : null,
        enemy: k >= 0 ? (drawn.current.enemies[k] ?? null) : null,
        building: world && pick.button === "right" ? buildingUnderRay(world, pick.ray) : null,
        ground: ground && [ground[0], ground[1]],
      });
    },
    [world, control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    select: (ids: number[]) => control.setSelected(ids),
    selected: () => control.selected,
    mode: () => control.mode,
    acks: () => control.acks,
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
  };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="readouts"
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={READOUTS_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
        onFrame={(project, distance) => readouts.current?.place(project, distance, drawnAt.current)}
        diagnostics={diagnostics}
      />
      <ReadoutLayer observation={observation} selected={control.selected} handle={readouts} />
      <aside className="lab-panel" data-testid="readouts-panel">
        <strong>Weapon readouts</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <CommandBar
          mode={control.mode}
          setMode={control.setMode}
          selected={control.selectedUnits}
          onStop={control.stop}
          onTogglePolicy={control.togglePolicy}
          onDeploy={control.setDeployment}
          onExit={control.exitBuilding}
        />
        <div className="lab-legend">
          Rings: <span className="lab-swatch lab-swatch-aim" /> aim ·{" "}
          <span className="lab-swatch lab-swatch-reload" /> reload (dashed) · number: rounds left (∞
          unlimited) · ⌖ guiding · lower badge: why it cannot fire · square: set-up (▲ setting up, ▼
          packing, ✓ set up)
        </div>
        <SelectionPanel units={control.selectedUnits} />
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
