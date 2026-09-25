// /lab/endurance (slice 16): the synthetic 100-a-side stress battle, played
// in real time with the production view, and live telemetry for the scale
// verdict. Stress input, clearly labelled: the village stays the play fixture.
import { useEffect, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import village from "@fixtures/village.json";
import { BattleView } from "../BattleView";
import { useBuiltScenario } from "../useBuiltScenario";
import type { BattleSession } from "../useBattleSession";

const CAMERA: Camera3DParams = {
  target: [1500, 1000, 0],
  distance: 2400,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};
const FRAME_WINDOW_MS = 10_000;

/** This page's frame intervals (ms) over the last ten seconds. */
function useFrameIntervals() {
  const intervals = useRef<{ at: number; ms: number }[]>([]);
  useEffect(() => {
    let raf = 0;
    let last = performance.now();
    const loop = (now: number) => {
      intervals.current.push({ at: now, ms: now - last });
      while (intervals.current[0].at < now - FRAME_WINDOW_MS) intervals.current.shift();
      last = now;
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, []);
  return intervals;
}

function quantiles(values: readonly number[]) {
  const v = [...values].sort((a, b) => a - b);
  const at = (q: number) => (v.length ? v[Math.round((v.length - 1) * q)] : 0);
  return { p50: at(0.5), p95: at(0.95), p99: at(0.99), max: v.length ? v[v.length - 1] : 0 };
}

/** The main thread's JS heap in MiB (Chromium only), else null. The
 * simulation worker's memory is not in it. */
function heapMiB(): number | null {
  const memory = (performance as { memory?: { usedJSHeapSize: number } }).memory;
  return memory ? memory.usedJSHeapSize / 2 ** 20 : null;
}

export default function Endurance() {
  const [late, setLate] = useState(false);
  const [seed, setSeed] = useState(1);
  const built = useBuiltScenario({ late, seed }, (wasm, o) =>
    wasm.endurance_scenario(JSON.stringify(village), BigInt(o.seed), o.late),
  );
  const frames = useFrameIntervals();
  // Repaint the telemetry once a second.
  const [, setBeat] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setBeat((b) => b + 1), 1000);
    return () => clearInterval(id);
  }, []);
  if (built && typeof built !== "string")
    return <main className="lab-rejected">{built.error}</main>;
  if (!built) return null;

  const telemetry = ({ sim, gpuAllocations }: BattleSession) => {
    const o = sim.latest.current;
    return {
      tick: o?.tick ?? 0,
      status: sim.status.status,
      slow: sim.status.slow,
      frames: quantiles(frames.current.map((f) => f.ms)),
      ownUnits: o?.own.length ?? 0,
      ownSoldiers: o?.own.reduce((n, u) => n + u.members.length, 0) ?? 0,
      corpsesSeen: o?.corpses.length ?? 0,
      projectilesSeen: o?.projectiles.length ?? 0,
      heapMiB: heapMiB(),
      gpu: gpuAllocations(),
    };
  };
  const panel = (session: BattleSession) => {
    const t = telemetry(session);
    const f = t.frames;
    return (
      <>
        <div className="lab-hint">Synthetic stress battle, not a play fixture.</div>
        <div className="lab-row">
          <label>
            <input type="checkbox" checked={late} onChange={(e) => setLate(e.target.checked)} />{" "}
            Late state (20,000 fallen, 2,000 wrecks)
          </label>
          <label>
            Seed{" "}
            <input
              type="number"
              value={seed}
              style={{ width: 70 }}
              onChange={(e) => setSeed(Math.max(0, Math.trunc(Number(e.target.value))) || 0)}
            />
          </label>
        </div>
        <ul className="lab-log" data-testid="telemetry">
          <li>
            tick {t.tick} ({(t.tick / village.tick_hz / 60).toFixed(1)} min) · {t.status}
            {t.slow ? " · behind real time" : ""}
          </li>
          <li>
            frame interval ms: p50 {f.p50.toFixed(1)} · p95 {f.p95.toFixed(1)} · p99{" "}
            {f.p99.toFixed(1)} · max {f.max.toFixed(0)}
          </li>
          <li>
            blue: {t.ownUnits} units, {t.ownSoldiers} soldiers · fallen seen {t.corpsesSeen} ·
            rounds in view {t.projectilesSeen}
          </li>
          <li>
            main-thread heap {t.heapMiB === null ? "n/a" : `${t.heapMiB.toFixed(0)} MiB`} · GPU
            buffers{" "}
            {t.gpu ? `${t.gpu.buffers} (${(t.gpu.bufferBytes / 2 ** 20).toFixed(1)} MiB)` : "n/a"}
          </li>
        </ul>
        <button type="button" onClick={session.sim.reset}>
          Reset
        </button>
      </>
    );
  };
  return (
    <BattleView
      key={`${late}-${seed}`}
      fixture="endurance"
      scenario={built}
      seed={seed}
      camera={CAMERA}
      title="Endurance (stress)"
      panel={panel}
      diagnostics={(session) => ({
        telemetry: () => telemetry(session),
        late: () => late,
        resetFrames: () => (frames.current = []),
      })}
    />
  );
}
