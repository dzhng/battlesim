// A played (or replayed) battle for blue: the world, the side's units and
// every overlay, the production readouts, selection panel and command bar.
// Routes compose it with their own panel content (the village's hold status
// and replay controls, the endurance lab's telemetry).
import { useCallback, useEffect, useMemo, useRef, type ReactNode } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  CommandBar,
  ReadoutLayer,
  SelectionPanel,
  type ReadoutLayerHandle,
} from "@web/battle/present/readouts";
import { Captions, SoundSwitch, useSoundCues } from "@web/battle/present/captions";
import type { ObservationView } from "@web/battle/sim/observation";
import { AckLog } from "./AckLog";
import { BattleMemory, buildBattleOverlay, type BattleOverlayScenario } from "./battleOverlay";
import { LabViewport } from "./LabViewport";
import { useBattleSession, type BattleSession } from "./useBattleSession";

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
  const memory = useRef(new BattleMemory());
  // Heard sounds pan by where the camera looks now.
  const yaw = useRef(camera.yaw);
  const cues = useSoundCues(() => yaw.current);
  const { note: noteCues } = cues;
  const onDecoded = useCallback(
    (o: ObservationView) => {
      memory.current.note(o);
      noteCues(o);
    },
    [noteCues],
  );
  const session = useBattleSession({
    map: parsed.map,
    scenario,
    seed,
    onDecoded,
    replay,
    buildings: "apart",
  });
  const { world, meshes, standing, sim, control, surfaceZ } = session;
  const { observation } = sim;
  const readouts = useRef<ReadoutLayerHandle>(null);
  const { clear: clearCues } = cues;
  useEffect(() => {
    memory.current.clear();
    clearCues();
  }, [sim.client, clearCues]);

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

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture={fixture}
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={session.frameInstances}
        initialCamera={camera}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={(project, view) => {
          yaw.current = view.yaw;
          readouts.current?.place(project, view.distance, session.drawnAt.current);
        }}
        diagnostics={{
          ...session.probes,
          transcript: () => cues.transcript.current,
          ...diagnostics?.(session),
        }}
      />
      <ReadoutLayer
        observation={observation}
        selected={control.selected}
        handle={readouts}
        groundZ={surfaceZ}
      />
      <aside className="lab-panel" data-testid="battle-panel">
        <strong>{title}</strong>
        {sim.error && (
          <div className="lab-rejected" data-testid="error">
            {sim.error}
          </div>
        )}
        {panel(session)}
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
        <SoundSwitch cues={cues} />
        <Captions cues={cues} />
        {!replay && <AckLog acks={control.acks} />}
      </aside>
    </>
  );
}
