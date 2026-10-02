// /lab/endurance: the synthetic 100-a-side stress battle, played
// in real time with the production view, and live telemetry for the scale
// verdict. Clearly labelled stress input, on its saved or full generated world.
import { useEffect, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import game from "@fixtures/game.json";
import { restartBattle } from "@web/mechanicsLifecycle";
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import recipes from "@fixtures/encounters.json?raw";
import { prepareBattle } from "@web/battle/prepare/client";
import { generationRequest } from "@web/maps/source";
import { GAME_RULES } from "../scenarios";
import { enduranceScenario } from "../savedMaps";
import { BattleView } from "../BattleView";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import type { BattleSession } from "../useBattleSession";
import { gameCamera } from "../gameCamera";

const CAMERA: Camera3DParams = {
  target: [1500, 1000, 0],
  distance: 2400,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
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
  const generated = new URLSearchParams(window.location.search).get("generated") === "1";
  const [late, setLate] = useState(
    () => new URLSearchParams(window.location.search).get("late") === "1",
  );
  const [seed, setSeed] = useState(() => {
    const requested = new URLSearchParams(window.location.search).get("seed");
    return requested === null
      ? generated
        ? 4
        : 1
      : Math.max(0, Math.trunc(Number(requested))) || 0;
  });
  const choose = (nextSeed: number, nextLate: boolean) =>
    restartBattle(
      () => {
        setSeed(nextSeed);
        setLate(nextLate);
      },
      { seed: String(nextSeed), late: nextLate ? "1" : "0" },
    );
  const built = useBuiltScenario({ late, seed, generated }, async (wasm, o, signal) => {
    if (!o.generated)
      return { scenario: await enduranceScenario(wasm, "endurance", o.seed, o.late), report: null };
    const preparation = prepareBattle(
      {
        type: "prepare",
        request: {
          map_source: {
            kind: "generated",
            request: generationRequest(
              wasm,
              { type: "metro", size: "large", seed: "4" },
              { presets, templates },
              config.limits,
            ),
          },
          recipe_id: config.encounter.recipe,
          encounter_seed: config.encounter.seed,
          battle_seed: o.seed,
        },
        documents: { presets, templates, recipes, rules: JSON.stringify(GAME_RULES) },
        stress: { kind: "city-arena-1", late: o.late },
      },
      () => {},
    );
    signal.addEventListener("abort", () => preparation.cancel(), { once: true });
    return preparation.battle;
  });
  const frames = useFrameIntervals();
  // Repaint the telemetry once a second.
  const [, setBeat] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setBeat((b) => b + 1), 1000);
    return () => clearInterval(id);
  }, []);
  if (buildFailed(built)) return <main className="lab-rejected">{built.error}</main>;
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
  // The synthetic stress battle's switches, in the pause menu.
  const menu = () => (
    <section className="hud-menu-section" aria-label="Stress battle">
      <label>
        <input type="checkbox" checked={late} onChange={(e) => choose(seed, e.target.checked)} />{" "}
        Late state (20,000 fallen, 2,000 wrecks)
      </label>
      <label>
        Seed{" "}
        <input
          type="number"
          value={seed}
          style={{ width: 70 }}
          onChange={(e) => choose(Math.max(0, Math.trunc(Number(e.target.value))) || 0, late)}
        />
      </label>
    </section>
  );
  // Its telemetry, in the top bar.
  const status = (session: BattleSession) => {
    const t = telemetry(session);
    const f = t.frames;
    return (
      <>
        <ul className="lab-log" data-testid="telemetry">
          <li>
            tick {t.tick} ({(t.tick / game.tick_hz / 60).toFixed(1)} min) · {t.status}
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
      </>
    );
  };
  return (
    <BattleView
      key={`${generated}-${late}-${seed}`}
      fixture="endurance"
      scenario={built.scenario}
      seed={seed}
      camera={
        built.report
          ? { ...CAMERA, target: [built.report.size[0] / 2, built.report.size[1] / 2, 0] }
          : CAMERA
      }
      status={status}
      menu={menu}
      diagnostics={(session) => ({
        telemetry: () => telemetry(session),
        late: () => late,
        preparation: () => built.report,
        resetFrames: () => (frames.current = []),
      })}
    />
  );
}
