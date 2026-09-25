import { useCallback, useEffect, useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { proxyForUnit, SIDE_COLORS } from "@packages/battle-renderer/src/unitProxies";
import { createSimClient } from "@web/battle/sim/client";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import { AckLine } from "../AckLine";
import type { Order } from "@web/battle/sim/protocol";
import geometryMap from "@fixtures/geometry-lab.json";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { groundUnderRay, useStaticWorld } from "../useStaticWorld";
import { useSimSession } from "../useSimSession";

// Blue only: enemy units stay absent until sensing produces permitted observations.
const SCENARIO = labScenario(geometryMap, [
  { side: "blue", kind: "tank", position: [60, 150] },
  { side: "blue", kind: "rifle", position: [50, 170] },
  { side: "blue", kind: "rifle", position: [50, 130] },
  { side: "blue", kind: "supply", position: [30, 150] },
]);
const SEED = 20260925;

export const AUTHORITY_CAMERA: Camera3DParams = {
  target: [58, 150, 0],
  distance: 62,
  pitch: 0.85,
  yaw: -1.35,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

type ReplayCheck =
  | { state: "running" }
  | { state: "match"; ticks: number }
  | { state: "mismatch"; tick: number };

export default function Authority() {
  const world = useStaticWorld(geometryMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  const sim = useSimSession({ scenario: SCENARIO, seed: SEED });
  const { client, observation, status } = sim;
  const [withhold, setWithhold] = useState(false);
  const [paused, setPaused] = useState(false);
  const [replayCheck, setReplayCheck] = useState<ReplayCheck | null>(null);
  const control = useUnitControl(client, observation);

  // A reset (new client) starts with fresh controls.
  useEffect(() => {
    setWithhold(false);
    setPaused(false);
    setReplayCheck(null);
  }, [client]);

  const toggleWithhold = (next: boolean) => {
    setWithhold(next);
    sim.holdCredit(next);
  };

  const togglePause = () => {
    if (paused) client?.resume();
    else client?.pause();
    setPaused(!paused);
  };

  /** Replays the accepted commands in-thread and compares every tick digest
   *  with what the worker published: same-build replay and worker/direct parity. */
  const checkReplay = useCallback(async (): Promise<ReplayCheck> => {
    const live = client!;
    setReplayCheck({ state: "running" });
    const json = await live.replay();
    const recorded = sim.digests.current;
    const last = Math.max(...recorded.keys());
    const replay = createSimClient({
      scenario: SCENARIO,
      seed: SEED,
      side: "blue",
      transport: "direct",
      replay: json,
    });
    let result: ReplayCheck = { state: "match", ticks: last };
    replay.onPublication((p) => {
      if (result.state === "match" && recorded.has(p.tick) && recorded.get(p.tick) !== p.digest) {
        result = { state: "mismatch", tick: p.tick };
      }
      p.release();
    });
    await replay.ready;
    replay.start();
    replay.pause();
    await replay.advance(last);
    replay.dispose();
    setReplayCheck(result);
    return result;
  }, [client, sim.digests]);

  const onPick = useCallback(
    (pick: LabPick) => {
      const ground = world && pick.button === "right" ? groundUnderRay(world.view, pick.ray) : null;
      control.onPointer({
        ...pick,
        unit: pick.instance >= 0 ? (observation?.own[pick.instance]?.id ?? null) : null,
        ground: ground && [ground[0], ground[1]],
      });
    },
    [world, control, observation],
  );

  const instances = useMemo<SceneInstance[]>(
    () =>
      (observation?.own ?? []).map((u) => ({
        kind: proxyForUnit(u.kind),
        x: u.position[0],
        y: u.position[1],
        z: u.position[2],
        yaw: u.yaw,
        color: SIDE_COLORS.blue,
        highlight: control.selected.includes(u.id),
      })),
    [observation, control.selected],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    status: () => client?.status,
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    acks: () => control.acks,
    command: (order: Order, queued = false) => control.issue(order, queued),
    setWithhold: toggleWithhold,
    checkReplay,
    advance: (n: number) => client!.advance(n),
    reset: sim.reset,
  };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="authority"
        world={meshes}
        instances={instances}
        initialCamera={AUTHORITY_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="authority-panel">
        <strong>Authority</strong>
        <div className="lab-hint">
          Click: select · Right-click: move · Shift+right-click: queue · Middle‑drag: orbit
        </div>
        <div data-testid="authority-status">
          Tick {observation?.tick ?? "—"} · {status.status}
          {status.slow ? " · running slow" : ""}
        </div>
        <div data-testid="selection">
          {control.selectedUnits.length === 0
            ? "No unit selected"
            : control.selectedUnits.map((u) => (
                <div key={u.id}>
                  {u.kind} #{u.id}:{" "}
                  {u.goal ? `moving to (${u.goal.map((v) => v.toFixed(0)).join(", ")})` : "holding"}
                  {u.queue.length ? `, ${u.queue.length} queued` : ""}
                </div>
              ))}
        </div>
        <div className="lab-row">
          <button type="button" aria-pressed={paused} onClick={togglePause}>
            {paused ? "Resume" : "Pause"}
          </button>
          <button
            type="button"
            disabled={!paused}
            title="Advance one tick while paused"
            onClick={() => void client?.advance(1)}
          >
            Step
          </button>
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
          <button type="button" onClick={() => void checkReplay()}>
            Check replay
          </button>
        </div>
        <label title="Stall test: stop handing ticks back to the simulation. It must wait rather than drop or queue ticks.">
          <input
            type="checkbox"
            checked={withhold}
            onChange={(e) => toggleWithhold(e.target.checked)}
          />
          Withhold rendered ticks (stall test)
        </label>
        {replayCheck && (
          <div data-testid="replay-check">
            {replayCheck.state === "running"
              ? "Replaying…"
              : replayCheck.state === "match"
                ? `Replay matches every tick through ${replayCheck.ticks}`
                : `Replay diverges at tick ${replayCheck.tick}`}
          </div>
        )}
        <div className="lab-hint">Commands, newest first</div>
        <ul className="lab-log" data-testid="ack-log">
          {control.acks.map((a) => (
            <AckLine key={a.seq} entry={a} />
          ))}
        </ul>
      </aside>
    </>
  );
}
