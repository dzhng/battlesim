import { useEffect, useRef, useState } from "react";
import initRenderer from "@wasm/renderer_spike/renderer_wasm_spike.js";
import rendererWasmUrl from "@wasm/renderer_spike/renderer_wasm_spike_bg.wasm?url";
import initSharedRenderer from "@wasm/renderer_browser/renderer_browser.js";
import sharedRendererWasmUrl from "@wasm/renderer_browser/renderer_browser_bg.wasm?url";
import { fullBattleBrowserRun, BATTLE_HEIGHT, BATTLE_WIDTH } from "./oneRendererBattle";
import {
  averageResults,
  formatMs,
  FRAMES,
  HEIGHT,
  LABELS,
  Mode,
  REAL_HEIGHT,
  REAL_WIDTH,
  REPEATS,
  Result,
  runMode,
  WIDTH,
} from "./oneRendererRuns";

export default function OneRenderer() {
  const [results, setResults] = useState<Result[]>([]);
  const [running, setRunning] = useState(false);
  const [wasmReady, setWasmReady] = useState(false);
  const wasmLoaded = useRef(false);
  const canvases = {
    typegpu: useRef<HTMLCanvasElement>(null),
    exported: useRef<HTMLCanvasElement>(null),
    rust: useRef<HTMLCanvasElement>(null),
    mixedRust: useRef<HTMLCanvasElement>(null),
    mixedOverlay: useRef<HTMLCanvasElement>(null),
    realBrowser: useRef<HTMLCanvasElement>(null),
    realRust: useRef<HTMLCanvasElement>(null),
    modelBrowser: useRef<HTMLCanvasElement>(null),
    modelRust: useRef<HTMLCanvasElement>(null),
    battleBrowser: useRef<HTMLCanvasElement>(null),
    battleRust: useRef<HTMLCanvasElement>(null),
    sharedCore: useRef<HTMLCanvasElement>(null),
  };

  useEffect(() => {
    if (wasmLoaded.current) return;
    wasmLoaded.current = true;
    void Promise.all([
      initRenderer({ module_or_path: rendererWasmUrl }),
      initSharedRenderer({ module_or_path: sharedRendererWasmUrl }),
    ]).then(() => setWasmReady(true));
  }, []);

  const run = async () => {
    setRunning(true);
    setResults([]);
    const allTasks: readonly [Mode, () => Promise<Result>][] = [
      ["typegpu", () => runMode("typegpu", canvases)],
      ["exported-wgsl", () => runMode("exported-wgsl", canvases)],
      ["rust-wgpu", () => runMode("rust-wgpu", canvases)],
      ["mixed", () => runMode("mixed", canvases)],
      ["real-browser", () => runMode("real-browser", canvases)],
      ["real-rust-wgpu", () => runMode("real-rust-wgpu", canvases)],
      ["model-browser", () => runMode("model-browser", canvases)],
      ["model-rust-wgpu", () => runMode("model-rust-wgpu", canvases)],
      [
        "battle-browser",
        () =>
          fullBattleBrowserRun(canvases.battleBrowser.current!, FRAMES).then((result) => ({
            ...result,
            mode: "battle-browser",
          })),
      ],
      ["battle-rust-wgpu", () => runMode("battle-rust-wgpu", canvases)],
      ["shared-rust-core-browser", () => runMode("shared-rust-core-browser", canvases)],
    ];
    const capture = new URLSearchParams(window.location.search).get("capture") === "1";
    const taskFactories = capture
      ? allTasks.filter(
          ([mode]) => mode === "battle-browser" || mode === "shared-rust-core-browser",
        )
      : allTasks;
    const samples = new Map<Mode, Result[]>();
    for (let repeat = 0; repeat < REPEATS; repeat++) {
      // Rotate the order between repeats so one adapter/browser scheduling
      // position cannot systematically favour one backend.
      const tasks = taskFactories
        .slice(repeat % taskFactories.length)
        .concat(taskFactories.slice(0, repeat % taskFactories.length));
      for (const [mode, task] of tasks) {
        let result: Result;
        try {
          result = await task();
        } catch (error) {
          result = {
            mode,
            setupMs: 0,
            encodeMs: 0,
            submitMs: 0,
            frames: 0,
            error: String(error),
          };
        }
        const modeSamples = samples.get(mode) ?? [];
        modeSamples.push(result);
        samples.set(mode, modeSamples);
      }
    }
    setResults(taskFactories.map(([mode]) => averageResults(mode, samples.get(mode)!)));
    setRunning(false);
  };

  return (
    <main style={{ padding: 24, maxWidth: 1100, margin: "0 auto", fontFamily: "system-ui" }}>
      <p style={{ opacity: 0.65, letterSpacing: "0.08em", textTransform: "uppercase" }}>
        Renderer spike
      </p>
      <h1>TypeGPU and Rust wgpu on the same WebGPU canvas contract</h1>
      <p>
        Eleven rows compare the fullscreen workload, a real presentation-fed terrain frame, an
        authored runtime tank mesh, the production BattleFrame pass graph, and a shared
        native/browser core. The production-shaped Rust row consumes the same battle inputs but
        intentionally exposes the passes that are not ported yet.
      </p>
      <p>
        WASM: <strong>{wasmReady ? "ready" : "loading"}</strong> · frames per row: {FRAMES} ·
        interleaved repeats: {REPEATS}
      </p>
      <button
        disabled={!wasmReady || running}
        onClick={() => void run()}
        style={{
          background: "#27384a",
          color: "#d7e9ff",
          border: "1px solid #54708e",
          borderRadius: 4,
          padding: "8px 12px",
          cursor: wasmReady && !running ? "pointer" : "default",
        }}
      >
        {running ? "Running…" : "Run comparison"}
      </button>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
          gap: 16,
          marginTop: 24,
        }}
      >
        {(
          [
            ["typegpu", canvases.typegpu, "TypeGPU"],
            ["exported-wgsl", canvases.exported, "Exported WGSL"],
            ["rust-wgpu", canvases.rust, "Rust wgpu"],
          ] as const
        ).map(([mode, ref, title]) => (
          <figure key={mode} style={{ margin: 0 }}>
            <figcaption>{title}</figcaption>
            <canvas ref={ref} width={WIDTH} height={HEIGHT} style={{ width: "100%" }} />
          </figure>
        ))}
        <figure style={{ margin: 0, position: "relative" }}>
          <figcaption>Mixed: Rust base + TypeGPU overlay (cyan corner = overlay pass)</figcaption>
          <canvas
            ref={canvases.mixedRust}
            width={WIDTH}
            height={HEIGHT}
            style={{ width: "100%" }}
          />
          <canvas
            ref={canvases.mixedOverlay}
            width={WIDTH}
            height={HEIGHT}
            style={{ position: "absolute", left: 0, top: 22, width: "100%", opacity: 0.7 }}
          />
        </figure>
        <figure style={{ margin: 0 }}>
          <figcaption>Real presentation terrain + browser WebGPU</figcaption>
          <canvas
            ref={canvases.realBrowser}
            width={REAL_WIDTH}
            height={REAL_HEIGHT}
            style={{ width: "100%" }}
          />
        </figure>
        <figure style={{ margin: 0 }}>
          <figcaption>Authored tank mesh + browser WebGPU</figcaption>
          <canvas
            ref={canvases.modelBrowser}
            width={REAL_WIDTH}
            height={REAL_HEIGHT}
            style={{ width: "100%" }}
          />
        </figure>
        <figure style={{ margin: 0 }}>
          <figcaption>Authored tank mesh + Rust wgpu WASM</figcaption>
          <canvas
            ref={canvases.modelRust}
            width={REAL_WIDTH}
            height={REAL_HEIGHT}
            style={{ width: "100%" }}
          />
        </figure>
        <figure style={{ margin: 0 }}>
          <figcaption>Real presentation terrain + Rust wgpu WASM</figcaption>
          <canvas
            ref={canvases.realRust}
            width={REAL_WIDTH}
            height={REAL_HEIGHT}
            style={{ width: "100%" }}
          />
        </figure>
        <figure style={{ margin: 0 }}>
          <figcaption>Production BattleFrame + browser WebGPU</figcaption>
          <canvas
            ref={canvases.battleBrowser}
            width={BATTLE_WIDTH}
            height={BATTLE_HEIGHT}
            style={{ width: "100%" }}
          />
        </figure>
        <figure style={{ margin: 0 }}>
          <figcaption>Battle inputs + Rust wgpu WASM (production-shaped graph)</figcaption>
          <canvas
            ref={canvases.battleRust}
            width={BATTLE_WIDTH}
            height={BATTLE_HEIGHT}
            style={{ width: "100%" }}
          />
        </figure>
        <figure style={{ margin: 0 }}>
          <figcaption>Shared native/browser Rust core + browser WASM</figcaption>
          <canvas
            ref={canvases.sharedCore}
            width={REAL_WIDTH}
            height={REAL_HEIGHT}
            style={{ width: "100%" }}
          />
        </figure>
      </div>
      {results.length > 0 && (
        <table style={{ marginTop: 24, borderCollapse: "collapse", width: "100%" }}>
          <thead>
            <tr>
              {[
                "Path",
                "Setup",
                "Encode + submit",
                "GPU drain",
                "Frames",
                "Workload",
                "Status",
              ].map((heading) => (
                <th
                  key={heading}
                  style={{ textAlign: "left", borderBottom: "1px solid #777", padding: 8 }}
                >
                  {heading}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {results.map((row) => (
              <tr key={row.mode} data-samples={JSON.stringify(row.samples)}>
                <td style={{ padding: 8 }}>{LABELS[row.mode]}</td>
                <td style={{ padding: 8 }}>{formatMs(row.setupMs)}</td>
                <td style={{ padding: 8 }}>{formatMs(row.encodeMs)}</td>
                <td style={{ padding: 8 }}>{formatMs(row.submitMs)}</td>
                <td style={{ padding: 8 }}>{row.frames}</td>
                <td style={{ padding: 8, fontSize: 12 }}>{row.workload ?? "—"}</td>
                <td style={{ padding: 8 }}>{row.error ?? "ok"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </main>
  );
}
