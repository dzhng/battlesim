// The lab's pass inspector (any lab route with `?inspect`): which stage of the
// battle frame is shown, and what the frame costs. Overlays on black and on
// white show exactly what the overlay pass lays over post; where the two
// agree, the overlay is opaque and its colour is its own.
import { useEffect, useState } from "react";
import {
  FRAME_VIEWS,
  type BattleFrame,
  type FrameStats,
  type FrameView,
} from "@packages/battle-renderer/src/scene";

const MIB = 2 ** 20;
const REFRESH_MS = 500;

export function PassInspector({
  frame,
  setView,
}: {
  frame: BattleFrame;
  setView: (view: FrameView) => void;
}) {
  const [stats, setStats] = useState<FrameStats>(() => frame.stats());
  useEffect(() => {
    const timer = setInterval(() => setStats(frame.stats()), REFRESH_MS);
    return () => clearInterval(timer);
  }, [frame]);
  const { gpu, memory, shadow } = stats;
  return (
    <aside className="hud-panel lab-panel lab-inspector" data-testid="pass-inspector">
      <strong>Pass inspector</strong>
      <label className="lab-row">
        View{" "}
        <select value={stats.view} onChange={(e) => setView(e.target.value as FrameView)}>
          {FRAME_VIEWS.map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      </label>
      <div>
        {stats.width}×{stats.height} · frame {stats.frames}
      </div>
      <div>
        GPU frame:{" "}
        {gpu
          ? `${gpu.meanMs.toFixed(2)} ms mean, ${gpu.p95Ms.toFixed(2)} p95`
          : "no timestamp-query"}
      </div>
      <div>
        Textures {(memory.textureBytes / MIB).toFixed(1)} MiB in {memory.textures} · buffers{" "}
        {(memory.bufferBytes / MIB).toFixed(1)} MiB in {memory.buffers}
      </div>
      <div>
        Shadow receivers {shadow.receiverRange.map((m) => m.toFixed(0)).join("–")} m ·{" "}
        {shadow.cascades} cascades
      </div>
    </aside>
  );
}
