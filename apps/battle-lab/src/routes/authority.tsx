import { useCallback, useEffect, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { createSimClient } from "@web/battle/sim/client";
import { AckLog } from "../AckLog";
import geometryMap from "@fixtures/geometry-lab.json";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";
import { useFeed } from "../feed";
import { villageCamera } from "../villageCamera";

// Blue only: enemy units stay absent until sensing produces permitted observations.
// Field works dropped south of the column at tick 1 (a sandbag line, a fence
// and a row of dragon's teeth) and the jeep show every appearance
// the battle draws, learned by sight like any other body.
const prop = (kind: string, center: [number, number], half: [number, number, number], yaw = 0) => ({
  tick: 1,
  add_prop: { kind, center, yaw, half_extents: half },
});
const SCENARIO = labScenario(
  geometryMap,
  [
    { side: "blue", kind: "tank", position: [60, 150] },
    { side: "blue", kind: "rifle", position: [50, 170] },
    { side: "blue", kind: "rifle", position: [50, 130] },
    { side: "blue", kind: "supply", position: [30, 150] },
    { side: "blue", kind: "jeep", position: [72, 172], yaw: -0.4 },
  ],
  [
    prop("sandbags", [48, 119], [3, 0.4, 0.5]),
    prop("fence", [70, 112], [0.1, 6, 0.6]),
    ...[0, 1, 2, 3, 4].map((k) => prop("tooth", [78, 106 + 2.4 * k], [0.6, 0.6, 0.6])),
  ],
);
const SEED = 20260925;

const AUTHORITY_CAMERA: Camera3DParams = {
  target: [58, 150, 0],
  distance: 62,
  pitch: 0.85,
  yaw: -1.35,
  ...villageCamera.lens,
};

type ReplayCheck =
  | { state: "running" }
  | { state: "match"; ticks: number }
  | { state: "mismatch"; tick: number };

export default function Authority() {
  const digests = useRef(new Map<number, string>());
  const session = useBattleSession({
    map: geometryMap,
    scenario: SCENARIO,
    seed: SEED,
    onDecoded: (o, digest) => digests.current.set(o.tick, digest),
  });
  const { meshes, sim, control } = session;
  const worldFeed = useFeed(meshes);
  const { client, observation, status } = sim;
  const [withhold, setWithhold] = useState(false);
  const [paused, setPaused] = useState(false);
  const [replayCheck, setReplayCheck] = useState<ReplayCheck | null>(null);

  // A reset (new client) starts with fresh controls.
  useEffect(() => {
    digests.current = new Map();
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
    const recorded = digests.current;
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
  }, [client]);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, setWithhold: toggleWithhold, checkReplay };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="authority"
        world={worldFeed}
        structures={session.structures}
        massing={session.massingFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={AUTHORITY_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="authority-panel">
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
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
