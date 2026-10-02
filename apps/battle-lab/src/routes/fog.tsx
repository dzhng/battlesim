// /lab/fog: sight-light fog over the village street
// (`streetScenario.ts`). The panel switches the frame between the
// live look and the seen/unseen debug mask, and the side it is drawn for; the
// probes let the scene measure the fog against the simulation's 8 m sweep, run
// the GPU lookup against its oracle vectors, and turn one eye's bearing
// without moving it.
import { useEffect, useMemo, useState } from "react";
import type { FogEye, FogInput } from "@packages/battle-renderer/src/frame/fogInputs";
import type { FogLookupParams, FogProbes } from "@packages/battle-renderer/src/frame/fogVisibility";
import { oracleAnswers, oracleVectors } from "@packages/battle-renderer/src/frame/fogOracle";
import type { FrameView } from "@packages/battle-renderer/src/scene";
import game from "@fixtures/game.json";
import { loadWasm } from "@web/battle/sim/module";
import type { ObservationView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { STREET_CAMERA, STREET_SEED, useStreetScenario } from "../streetScenario";
import { gameFogGeometry } from "../gameFog";
import { useFeed } from "../feed";
import { TickStatus } from "../TickStatus";

/** Every 8 m fog cell's centre on the ground, and the simulation's bit there. */
function cellCentres(o: ObservationView, heightAt: (x: number, y: number) => number | undefined) {
  const { cellM, nx, ny, bits } = o.fog;
  const sim = new Int8Array(nx * ny).fill(-1);
  const points: { position: [number, number, number] }[] = [];
  const index: number[] = [];
  for (let j = 0; j < ny; j++) {
    for (let i = 0; i < nx; i++) {
      const x = (i + 0.5) * cellM;
      const y = (j + 0.5) * cellM;
      const z = heightAt(x, y);
      const k = j * nx + i;
      sim[k] = (bits[k >> 5] >>> (k & 31)) & 1;
      if (z === undefined) continue;
      points.push({ position: [x, y, z] });
      index.push(k);
    }
  }
  return { nx, ny, sim, points, index };
}

/**
 * GPU fog at every 8 m cell centre against the simulation's published field,
 * outside a one-cell band around the simulation's own boundary (spike 02's
 * metric): what remains is the sweep's 8 m quantisation.
 */
async function agreement(
  probes: FogProbes,
  o: ObservationView,
  heightAt: (x: number, y: number) => number | undefined,
) {
  const { nx, ny, sim, points, index } = cellCentres(o, heightAt);
  const seen = await probes.probe(points);
  const gpu = new Int8Array(nx * ny).fill(-1);
  index.forEach((k, n) => (gpu[k] = seen[n]));
  const bit = (i: number, j: number) =>
    i < 0 || j < 0 || i >= nx || j >= ny ? -1 : sim[j * nx + i];
  let band = 0;
  let outside = 0;
  let falseSeen = 0;
  let falseHidden = 0;
  let simSeen = 0;
  const diff: number[] = [];
  for (let j = 0; j < ny; j++) {
    for (let i = 0; i < nx; i++) {
      const k = j * nx + i;
      if (gpu[k] < 0) continue;
      const s = sim[k];
      let edge = false;
      for (let dj = -1; dj <= 1 && !edge; dj++)
        for (let di = -1; di <= 1; di++) {
          const b = bit(i + di, j + dj);
          if (b !== -1 && b !== s) edge = true;
        }
      if (edge) {
        band++;
        continue;
      }
      outside++;
      simSeen += s;
      if (gpu[k] !== s) {
        diff.push(k);
        if (gpu[k]) falseSeen++;
        else falseHidden++;
      }
    }
  }
  return {
    cells: nx * ny,
    band,
    outside,
    simSeen,
    falseSeen,
    falseHidden,
    percent: (100 * (falseSeen + falseHidden)) / Math.max(1, outside),
    gpuSeen: seen.reduce((n, v) => n + v, 0),
    /** Disagreeing cells: `k = j·nx + i`, for the evidence map. */
    diff,
    nx,
    ny,
  };
}

/** The GPU lookup against its CPU mirror on seeded synthetic maps. */
async function lookupOracle(probes: FogProbes, seed: number) {
  const lookup: FogLookupParams = {
    azimuthBins: 64,
    radialBins: 16,
    firstBinM: gameFogGeometry.first_bin_m,
    targetHeightM: game.sensors.fog_target_height_m,
    faceProbeM: gameFogGeometry.face_probe_m,
    foliageFullBlock: game.sensors.foliage_full_block,
  };
  const vectors = oracleVectors(seed, lookup);
  const cpu = oracleAnswers(lookup, vectors);
  const gpu = await probes.probeWith(lookup, vectors.eyes, vectors.maps, vectors.points);
  // Float noise decides nothing: skip vectors that sit on a comparison.
  const SETTLED = 1e-3;
  let compared = 0;
  let seen = 0;
  const mismatches: number[] = [];
  cpu.forEach((c, i) => {
    if (c.margin < SETTLED) return;
    compared++;
    seen += c.seen ? 1 : 0;
    if (gpu[i] !== (c.seen ? 1 : 0)) mismatches.push(i);
  });
  return {
    vectors: cpu.length,
    compared,
    seen,
    mismatches: mismatches.length,
    first: mismatches.slice(0, 5),
  };
}

/** The WGSL sight shape against `sim::sight::multiplier`'s oracle vectors. */
async function shapeOracle(probes: FogProbes) {
  const wasm = await loadWasm();
  const rows = wasm.sight_multiplier_vectors();
  const n = rows.length / 5;
  const input = new Float32Array(n * 4);
  for (let i = 0; i < n; i++) input.set(rows.subarray(i * 5, i * 5 + 4), i * 4);
  const gpu = await probes.probeShape(input);
  let maxError = 0;
  for (let i = 0; i < n; i++) maxError = Math.max(maxError, Math.abs(gpu[i] - rows[i * 5 + 4]));
  return { vectors: n, maxError };
}

export default function Fog() {
  const built = useStreetScenario("fog");
  if (built && typeof built !== "string")
    return <main className="lab-rejected">{built.error}</main>;
  if (!built) return null;
  return <FogLab scenario={built} />;
}

function FogLab({ scenario }: { scenario: string }) {
  const [side, setSide] = useState<SideName>("blue");
  const session = useBattleSession({
    scenario,
    seed: STREET_SEED,
    destroyable: "apart",
    side,
  });
  const { meshes, sim, world } = session;
  useEffect(() => sim.client?.observeAs(side), [sim.client, side]);
  const { observation } = sim;
  const [view, setView] = useState<FrameView>("final");
  /** Eyes the scene substitutes for the published ones (a turned turret). */
  const [eyes, setEyes] = useState<FogEye[] | null>(null);
  const fog = useMemo<FogInput | null>(
    () =>
      session.fog && eyes ? { ...session.fog, sight: { ...session.fog.sight, eyes } } : session.fog,
    [session.fog, eyes],
  );
  const fogFeed = useFeed(fog);
  const probes = () => window.__lab!.fog!();
  const heightAt = (x: number, y: number) => world?.view.height_at(x, y) ?? undefined;
  const show = (next: FrameView) => {
    setView(next);
    void window.__lab?.setFrameView?.(next);
  };

  const diagnostics = {
    ...session.probes,
    fogInput: () => fog,
    setEyes,
    // Sent now, so a following advance already publishes the new side.
    setSide: (next: SideName) => {
      sim.client?.observeAs(next);
      setSide(next);
    },
    agreement: () => agreement(probes(), sim.latest.current!, heightAt),
    lookupOracle: (seed: number) => lookupOracle(probes(), seed),
    shapeOracle: () => shapeOracle(probes()),
    probe: (points: Parameters<FogProbes["probe"]>[0]) => probes().probe(points),
    showMask: (on: boolean) => show(on ? "fog-mask" : "final"),
  };

  // The sight edge's geometry is measured on bare ground: its stair and
  // position checks trace the mask's edge line, and blades standing in
  // unseen ground rise over the seen field behind it, which is right but
  // moves that line by blade heights. The fog-look lab and the village draw
  // grass under fog.
  const bare = useMemo(() => meshes && { ...meshes, grass: null }, [meshes]);
  const worldFeed = useFeed(bare);
  if (!bare) return null;
  const stats = window.__lab?.stats?.().fog;
  return (
    <>
      <LabViewport
        fixture="fog"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        fog={fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={STREET_CAMERA}
        groundAt={session.surfaceZ}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="fog-panel">
        <strong>Fog geometry</strong>
        <div className="lab-hint">
          Blue's sight, per pixel: each eye's horizon map, cut by the terrain and the buildings blue
          knows.
        </div>
        <div className="lab-row">
          <button type="button" aria-pressed={view === "final"} onClick={() => show("final")}>
            Live frame
          </button>
          <button type="button" aria-pressed={view === "fog-mask"} onClick={() => show("fog-mask")}>
            Seen / unseen mask
          </button>
        </div>
        <div className="lab-row">
          {(["blue", "red"] as const).map((s) => (
            <button key={s} type="button" aria-pressed={side === s} onClick={() => setSide(s)}>
              {s === "blue" ? "Blue's sight" : "Red's sight"}
            </button>
          ))}
        </div>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        {stats && (
          <div className="lab-hint" data-testid="fog-stats">
            {stats.eyes} eyes · {stats.occluders} occluders · maps{" "}
            {((stats.mapBytes + stats.terrainMapBytes) / 2 ** 20).toFixed(1)} MiB
          </div>
        )}
      </aside>
    </>
  );
}
