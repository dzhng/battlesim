import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { proxyForUnit, SIDE_COLORS } from "@packages/battle-renderer/src/unitProxies";
import { createSimClient, type Publication, type SimClient } from "@web/battle/sim/client";
import type { ObservationView } from "@web/battle/sim/observation";
import type { AuthorityStatus, CommandAck, Order } from "@web/battle/sim/protocol";
import geometryMap from "@fixtures/geometry-lab.json";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { groundUnderRay, useStaticWorld } from "../useStaticWorld";

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

interface AckEntry {
  seq: number;
  order: string;
  ack: CommandAck | null;
}

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
  const [generation, setGeneration] = useState(0);
  const clientRef = useRef<SimClient | null>(null);
  const [observation, setObservation] = useState<ObservationView | null>(null);
  const [status, setStatus] = useState<{ status: AuthorityStatus; slow: boolean }>({
    status: "loading",
    slow: false,
  });
  const [acks, setAcks] = useState<AckEntry[]>([]);
  const [selected, setSelected] = useState<number[]>([]);
  const [withhold, setWithhold] = useState(false);
  const [paused, setPaused] = useState(false);
  const [replayCheck, setReplayCheck] = useState<ReplayCheck | null>(null);
  const withheld = useRef<Publication[]>([]);
  const observationRef = useRef<ObservationView | null>(null);
  observationRef.current = observation;
  const withholdRef = useRef(withhold);
  withholdRef.current = withhold;
  const digests = useRef(new Map<number, string>());
  const viewportReady = useRef(false);

  // One client per generation; Reset disposes it and starts again from the seed.
  useEffect(() => {
    const client = createSimClient({
      scenario: SCENARIO,
      seed: SEED,
      side: "blue",
      transport: "worker",
    });
    clientRef.current = client;
    digests.current = new Map();
    withheld.current = [];
    setAcks([]);
    setObservation(null);
    setReplayCheck(null);
    client.onStatus((next, slow) => setStatus({ status: next, slow }));
    client.onPublication((publication) => {
      digests.current.set(publication.tick, publication.digest);
      setObservation(publication.observation);
      if (withholdRef.current) withheld.current.push(publication);
      // Consumed once this frame has drawn it.
      else requestAnimationFrame(() => publication.release());
    });
    void client.ready.then(() => viewportReady.current && client.start());
    return () => client.dispose();
  }, [generation]);

  const onViewportReady = useCallback(() => {
    viewportReady.current = true;
    const client = clientRef.current;
    if (client) void client.ready.then(() => client.start());
  }, []);

  const unitName = useCallback((id: number) => {
    const unit = observationRef.current?.own.find((u) => u.id === id);
    return unit ? `${unit.kind} #${id}` : `unit #${id}`;
  }, []);

  const issue = useCallback(
    async (order: Order, queued: boolean) => {
      const client = clientRef.current;
      if (!client) return null;
      const target =
        order.kind === "move" ? ` to (${order.goal.map((v) => v.toFixed(0)).join(", ")})` : "";
      const entry: AckEntry = {
        seq: 0,
        order: `${order.kind} ${order.units.map(unitName).join(", ")}${target}${queued ? " (queued)" : ""}`,
        ack: null,
      };
      const ack = await client.command(order, queued);
      entry.seq = ack.seq;
      entry.ack = ack;
      setAcks((log) => [entry, ...log].slice(0, 8));
      return ack;
    },
    [unitName],
  );

  const toggleWithhold = (next: boolean) => {
    setWithhold(next);
    if (!next) for (const p of withheld.current.splice(0)) p.release();
  };

  const togglePause = () => {
    const client = clientRef.current!;
    if (paused) client.resume();
    else client.pause();
    setPaused(!paused);
  };

  /** Replays the accepted commands in-thread and compares every tick digest
   *  with what the worker published: same-build replay and worker/direct parity. */
  const checkReplay = useCallback(async (): Promise<ReplayCheck> => {
    const live = clientRef.current!;
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
  }, []);

  const onPick = useCallback(
    (pick: LabPick) => {
      if (!observation || !world) return;
      if (pick.button === "left") {
        const unit = pick.instance >= 0 ? observation.own[pick.instance].id : null;
        setSelected((current) =>
          unit === null
            ? pick.shift
              ? current
              : []
            : pick.shift
              ? [...new Set([...current, unit])]
              : [unit],
        );
        return;
      }
      const ground = groundUnderRay(world.view, pick.ray);
      if (!ground || selected.length === 0) return;
      void issue(
        {
          kind: "move",
          units: selected,
          gesture: Math.round(pick.time),
          goal: [ground[0], ground[1]],
          route: "shortest",
        },
        pick.shift,
      );
    },
    [observation, world, selected, issue],
  );

  const selectedUnits = (observation?.own ?? []).filter((u) => selected.includes(u.id));

  const instances = useMemo<SceneInstance[]>(
    () =>
      (observation?.own ?? []).map((u) => ({
        kind: proxyForUnit(u.kind),
        x: u.position[0],
        y: u.position[1],
        z: u.position[2],
        yaw: u.yaw,
        color: SIDE_COLORS.blue,
        highlight: selected.includes(u.id),
      })),
    [observation, selected],
  );

  const diagnostics = useMemo(
    () => ({
      status: () => clientRef.current?.status,
      tick: () => observation?.tick ?? 0,
      observation: () => observation,
      acks: () => acks,
      command: (order: Order, queued = false) => issue(order, queued),
      setWithhold: toggleWithhold,
      checkReplay,
      advance: (n: number) => clientRef.current!.advance(n),
      reset: () => setGeneration((g) => g + 1),
    }),
    // toggleWithhold only closes over refs and setters.
    [observation, acks, issue, checkReplay],
  );

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="authority"
        world={meshes}
        instances={instances}
        initialCamera={AUTHORITY_CAMERA}
        onPick={onPick}
        onReady={onViewportReady}
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
          {selectedUnits.length === 0
            ? "No unit selected"
            : selectedUnits.map((u) => (
                <div key={u.id}>
                  {u.kind} #{u.id}:{" "}
                  {u.goal ? `moving to (${u.goal.map((v) => v.toFixed(0)).join(", ")})` : "holding"}
                  {u.queued ? `, ${u.queued} queued` : ""}
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
            onClick={() => void clientRef.current?.advance(1)}
          >
            Step
          </button>
          <button type="button" onClick={() => setGeneration((g) => g + 1)}>
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
          {acks.map((a) => (
            <li key={a.seq} className={a.ack?.error ? "lab-rejected" : "lab-accepted"}>
              {a.ack?.error
                ? `✕ rejected (${a.ack.error.reason.replaceAll("_", " ")})`
                : `✓ accepted, applied at tick ${a.ack?.applied_tick}`}{" "}
              — #{a.seq} {a.order}
            </li>
          ))}
        </ul>
      </aside>
    </>
  );
}
