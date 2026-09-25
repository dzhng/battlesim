// A played (or replayed) battle for blue: the world, the side's units and
// every overlay, the production readouts, selection panel and command bar.
// Routes compose it with their own panel content (the village's hold status
// and replay controls, the endurance lab's telemetry).
import { useCallback, useEffect, useMemo, useRef, type ReactNode } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildStandingStructures, buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import {
  CommandBar,
  ReadoutLayer,
  SelectionPanel,
  type ReadoutLayerHandle,
} from "@web/battle/present/readouts";
import type { ObservationView } from "@web/battle/sim/observation";
import { AckLine } from "./AckLine";
import { BattleMemory, buildBattleOverlay, type BattleOverlayScenario } from "./battleOverlay";
import { LabViewport, type LabPick } from "./LabViewport";
import { sideInstances } from "./sideInstances";
import { useSimSession } from "./useSimSession";
import { buildingUnderRay, groundUnderRay, useStaticWorld } from "./useStaticWorld";

export type BattleSession = ReturnType<typeof useSimSession>;
export type BattleControl = ReturnType<typeof useUnitControl>;

export function BattleView({
  fixture,
  scenario,
  seed,
  replay,
  camera,
  title,
  panel,
  diagnostics,
}: {
  fixture: string;
  /** The scenario JSON the authority runs; the view draws its map. */
  scenario: string;
  seed: number;
  /** A recorded battle to replay: input is off. */
  replay?: string;
  camera: Camera3DParams;
  title: string;
  /** Route panel content under the title. */
  panel: (session: BattleSession) => ReactNode;
  /** Route-specific lab probes, merged into the shared ones. */
  diagnostics?: (session: BattleSession) => Record<string, unknown>;
}) {
  const parsed = useMemo(() => {
    const s = JSON.parse(scenario) as {
      map: unknown;
      rules: { service: { radius_m: number } };
      encounter: { success_zone_center: [number, number]; success_zone_radius_m: number } | null;
    };
    const drawn: BattleOverlayScenario = {
      supplyRadius: s.rules.service.radius_m,
      zone: s.encounter && {
        center: s.encounter.success_zone_center,
        radius: s.encounter.success_zone_radius_m,
      },
    };
    return { map: s.map, drawn };
  }, [scenario]);
  const world = useStaticWorld(parsed.map);
  const memory = useRef(new BattleMemory());
  const onDecoded = useCallback((o: ObservationView) => memory.current.note(o), []);
  const sim = useSimSession({ scenario, seed, onDecoded, replay });
  const { observation } = sim;
  const control = useUnitControl(replay ? null : sim.client, observation);
  const drawn = useRef<{ owners: (number | null)[]; enemies: (number | null)[] }>({
    owners: [],
    enemies: [],
  });
  const drawnAt = useRef(new Map<number, readonly [number, number, number]>());
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;
  const readouts = useRef<ReadoutLayerHandle>(null);
  useEffect(() => memory.current.clear(), [sim.client]);

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
      const d = sideInstances("blue", poses, observation, selectedRef.current);
      drawn.current = d;
      drawnAt.current = new Map(poses.map((p) => [p.id, p.position]));
      return d.instances;
    },
    [observation, sim.interpolator],
  );
  const overlay = useMemo(
    () =>
      world && observation && standing
        ? buildBattleOverlay(
            observation,
            memory.current,
            control.selected,
            standing,
            surfaceZ,
            parsed.drawn,
          )
        : undefined,
    [world, observation, standing, surfaceZ, control.selected, parsed.drawn],
  );
  const onPick = useCallback(
    (pick: LabPick) => {
      if (!world) return;
      const right = pick.button === "right";
      const ground = right ? groundUnderRay(world.view, pick.ray) : null;
      const k = pick.instance;
      control.onPointer({
        ...pick,
        unit: k >= 0 ? (drawn.current.owners[k] ?? null) : null,
        enemy: k >= 0 ? (drawn.current.enemies[k] ?? null) : null,
        building: right ? buildingUnderRay(world, pick.ray) : null,
        ground: ground && [ground[0], ground[1]],
      });
    },
    [world, control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const probes = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    digest: (tick: number) => sim.digests.current.get(tick),
    error: () => sim.error,
    status: () => sim.status,
    selected: () => control.selected,
    select: (ids: number[]) => control.setSelected(ids),
    acks: () => control.acks,
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
    ...diagnostics?.(sim),
  };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture={fixture}
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={camera}
        onPick={onPick}
        onReady={sim.onViewportReady}
        onFrame={(project, distance) => readouts.current?.place(project, distance, drawnAt.current)}
        diagnostics={probes}
      />
      <ReadoutLayer observation={observation} selected={control.selected} handle={readouts} />
      <aside className="lab-panel" data-testid="battle-panel">
        <strong>{title}</strong>
        {sim.error && (
          <div className="lab-rejected" data-testid="error">
            {sim.error}
          </div>
        )}
        {panel(sim)}
        {!replay && (
          <CommandBar
            mode={control.mode}
            setMode={control.setMode}
            selected={control.selectedUnits}
            onStop={control.stop}
            onTogglePolicy={control.togglePolicy}
            onDeploy={control.setDeployment}
            onExit={control.exitBuilding}
          />
        )}
        <SelectionPanel units={control.selectedUnits} />
        {!replay && (
          <>
            <div className="lab-hint">Commands, newest first</div>
            <ul className="lab-log" data-testid="ack-log">
              {control.acks.length === 0 && <li>None yet</li>}
              {control.acks.map((a) => (
                <AckLine key={a.seq} entry={a} />
              ))}
            </ul>
          </>
        )}
      </aside>
    </>
  );
}
