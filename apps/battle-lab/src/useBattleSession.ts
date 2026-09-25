// One side's live battle session: the static world and its meshes, the worker
// authority, the player command path, the drawn units (interpolated, one
// instance per soldier), the pick and box-select adapters over what is drawn,
// and the base lab probes. The battle view and every lab that plays a battle
// share it; routes add only what they show.
import { useCallback, useMemo, useRef } from "react";
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import { buildStandingStructures, buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { ObservationView } from "@web/battle/sim/observation";
import type { Order, SideName } from "@web/battle/sim/protocol";
import type { LabBox, LabPick, ViewportGpu } from "./LabViewport";
import { pickToPointer, sideInstances, type DrawnInstances } from "./sideInstances";
import { useSimSession } from "./useSimSession";
import { useStaticWorld } from "./useStaticWorld";

type P3 = readonly [number, number, number];

export interface BattleSessionOptions {
  /** The map the scenario runs on (the scenario's own `map`). */
  map: unknown;
  /** The scenario JSON the authority runs. */
  scenario: string;
  seed: number;
  /** Called for every decoded frame, before its credit returns. */
  onDecoded?: (o: ObservationView) => void;
  /** A recorded battle to replay: input is off. */
  replay?: string;
  /** Whose units and identified enemies are drawn (a lab's diagnostic side switch). */
  side?: SideName;
  /** "apart": buildings are drawn apart from the world, so one seen to fall
   *  leaves the map and its known ruin stands in its place. */
  buildings?: "apart";
}

/** The rule values the scenario runs under (only what views read). */
export interface ScenarioRules {
  service: { radius_m: number; deploy_and_pack_s: number; stock: number };
}

export function useBattleSession({
  map,
  scenario,
  seed,
  onDecoded,
  replay,
  side = "blue",
  buildings,
}: BattleSessionOptions) {
  const world = useStaticWorld(map);
  const rules = useMemo(() => (JSON.parse(scenario) as { rules: ScenarioRules }).rules, [scenario]);
  const sim = useSimSession({ scenario, seed, onDecoded, replay });
  const { observation } = sim;
  const control = useUnitControl(replay ? null : sim.client, observation);

  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface", buildings),
    [world, buildings],
  );
  const fallenKey = (observation?.knownProps ?? [])
    .flatMap((p) => (p.replaces === null ? [] : [p.replaces]))
    .join();
  const standing = useMemo(
    () =>
      world && buildings === "apart"
        ? buildStandingStructures(
            world.exports,
            world.layout,
            new Set(fallenKey ? fallenKey.split(",").map(Number) : []),
          )
        : null,
    [world, buildings, fallenKey],
  );
  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );

  // What the last frame drew: which unit or enemy each instance is, and each
  // own unit's drawn (interpolated) position.
  const drawn = useRef<DrawnInstances>({ instances: [], owners: [], enemies: [] });
  const drawnAt = useRef(new Map<number, P3>());
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;
  const sideRef = useRef(side);
  sideRef.current = side;
  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      const d = sideInstances(sideRef.current, poses, observation, selectedRef.current);
      drawn.current = d;
      drawnAt.current = new Map(poses.map((p) => [p.id, p.position]));
      return d.instances;
    },
    [observation, sim.interpolator],
  );

  const onPick = useCallback(
    (pick: LabPick) => {
      if (world) control.onPointer(pickToPointer(world, drawn.current, pick));
    },
    [world, control],
  );
  /** Drag-select own units whose drawn position falls in the rectangle. */
  const onBox = useCallback(
    (box: LabBox) =>
      control.selectInRect((u) => {
        const at = drawnAt.current.get(u.id) ?? u.position;
        const p = box.project(at[0], at[1], at[2] + 1);
        return !!p && p[0] >= box.x0 && p[0] <= box.x1 && p[1] >= box.y0 && p[1] <= box.y1;
      }, box.shift),
    [control],
  );

  // The viewport's GPU allocation counter, once it is up.
  const gpu = useRef<ViewportGpu | null>(null);
  const { onViewportReady } = sim;
  const onReady = useCallback(
    (viewport: ViewportGpu) => {
      gpu.current = viewport;
      onViewportReady();
    },
    [onViewportReady],
  );
  const gpuAllocations = useCallback(
    (): GpuAllocationCounts | null => gpu.current?.allocations() ?? null,
    [],
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
    command: (order: Order, queued = false) => control.issue(order, queued),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
    reset: () => sim.reset(),
  };

  return {
    world,
    meshes,
    /** The buildings still standing, drawn apart ("apart" only). */
    standing,
    rules,
    sim,
    control,
    surfaceZ,
    frameInstances,
    drawnAt,
    onPick,
    onBox,
    onReady,
    gpuAllocations,
    probes,
  };
}

export type BattleSession = ReturnType<typeof useBattleSession>;
