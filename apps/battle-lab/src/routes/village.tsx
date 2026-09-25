import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildStandingStructures, buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import {
  CommandBar,
  ReadoutLayer,
  SelectionPanel,
  type ReadoutLayerHandle,
} from "@web/battle/present/readouts";
import type { ObservationView } from "@web/battle/sim/observation";
import village from "@fixtures/village.json";
import { AckLine } from "../AckLine";
import { BattleMemory, buildBattleOverlay } from "../battleOverlay";
import { LabViewport, type LabPick } from "../LabViewport";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { buildingUnderRay, groundUnderRay, useStaticWorld } from "../useStaticWorld";
import { loadWasm } from "../wasm";

type Variant = "ordinary" | "prepared_crossfire";
const VARIANT_LABEL: Record<Variant, string> = {
  ordinary: "Ordinary ambush",
  prepared_crossfire: "Prepared crossfire",
};
/** A saved battle: the variant it was played on and every accepted command. */
interface ReplayFile {
  variant: Variant;
  replay: string;
}
const LAST_REPLAY_KEY = "village-last-replay";

// Blue's start and the village both in view, right of the panel.
export const VILLAGE_CAMERA: Camera3DParams = {
  target: [360, 800, 0],
  distance: 1150,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

const HOLD_S = village.encounter.hold_s;
const TICK_HZ = village.tick_hz;
const RESULT_TEXT: Record<string, string> = {
  running: "in progress",
  captured: "village captured",
  defeated: "blue has no combat units left",
  inconclusive: `${village.encounter.max_assessment_s / 60} minutes passed: inconclusive (play on)`,
};

const isVariant = (v: unknown): v is Variant => typeof v === "string" && v in VARIANT_LABEL;

/** The village scenario JSON for a variant, built by the simulation, tagged
 * with its variant so a view never pairs one variant's battle with another's. */
function useVillageScenario(variant: Variant): { variant: Variant; json: string } | string | null {
  const [scenario, setScenario] = useState<{ variant: Variant; json: string } | string | null>(
    null,
  );
  useEffect(() => {
    let live = true;
    setScenario(null);
    loadWasm()
      .then((wasm) => ({ variant, json: wasm.village_scenario(JSON.stringify(village), variant) }))
      .then(
        (built) => live && setScenario(built),
        (e: Error) => live && setScenario(`the village scenario could not be built: ${e.message}`),
      );
    return () => {
      live = false;
    };
  }, [variant]);
  return scenario;
}

/** The scenario for `variant` once built; an error message if building failed. */
function scenarioFor(
  built: ReturnType<typeof useVillageScenario>,
  variant: Variant,
): string | { error: string } | null {
  if (typeof built === "string") return { error: built };
  return built?.variant === variant ? built.json : null;
}

function Failed({ error }: { error: string }) {
  return (
    <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
      {error}
    </main>
  );
}

function replaySeed(file: ReplayFile): string {
  try {
    return String((JSON.parse(file.replay) as { seed: number }).seed);
  } catch {
    return "?";
  }
}

function readSavedReplay(): ReplayFile | null {
  try {
    const raw = localStorage.getItem(LAST_REPLAY_KEY);
    const file = raw ? (JSON.parse(raw) as ReplayFile) : null;
    return file && isVariant(file.variant) ? file : null;
  } catch {
    return null;
  }
}

/** /battle/village: play the encounter. */
export default function VillageBattle() {
  const [variant, setVariant] = useState<Variant>("ordinary");
  const [seed, setSeed] = useState<number>(village.seed);
  const scenario = scenarioFor(useVillageScenario(variant), variant);
  if (!scenario) return null;
  if (typeof scenario !== "string") return <Failed error={scenario.error} />;
  return (
    <VillageView
      key={`${variant}-${seed}`}
      scenario={scenario}
      seed={seed}
      variant={variant}
      setVariant={setVariant}
      setSeed={setSeed}
    />
  );
}

/** /replay/village: watch a saved battle; input is off, the defender is off. */
export function VillageReplay() {
  // Each loaded file gets a fresh view, even one identical to the last.
  const [loaded, setLoaded] = useState<{ file: ReplayFile | null; n: number }>(() => ({
    file: readSavedReplay(),
    n: 0,
  }));
  const setFile = (file: ReplayFile) => setLoaded((l) => ({ file, n: l.n + 1 }));
  const { file } = loaded;
  const scenario = scenarioFor(
    useVillageScenario(file?.variant ?? "ordinary"),
    file?.variant ?? "ordinary",
  );
  if (!file)
    return (
      <main style={{ padding: 24 }}>
        <h1>Village replay</h1>
        <ReplayImport onLoad={setFile} />
      </main>
    );
  if (!scenario) return null;
  if (typeof scenario !== "string") return <Failed error={scenario.error} />;
  return (
    <VillageView
      key={loaded.n}
      scenario={scenario}
      seed={0}
      variant={file.variant}
      replay={file}
      onLoadReplay={setFile}
    />
  );
}

function ReplayImport({ onLoad }: { onLoad: (file: ReplayFile) => void }) {
  const [error, setError] = useState<string | null>(null);
  return (
    <label className="lab-hint">
      Load a saved village battle (.json):{" "}
      <input
        type="file"
        accept="application/json"
        data-testid="replay-file"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (!f) return;
          void f.text().then((text) => {
            try {
              const parsed = JSON.parse(text) as ReplayFile;
              if (typeof parsed.replay !== "string" || !isVariant(parsed.variant))
                throw new Error("not a village battle file");
              setError(null);
              onLoad(parsed);
            } catch (err) {
              setError((err as Error).message);
            }
          });
        }}
      />
      {error && <span className="lab-rejected"> {error}</span>}
    </label>
  );
}

function VillageView({
  scenario,
  seed,
  variant,
  setVariant,
  setSeed,
  replay,
  onLoadReplay,
}: {
  scenario: string;
  seed: number;
  variant: Variant;
  setVariant?: (v: Variant) => void;
  setSeed?: (s: number) => void;
  replay?: ReplayFile;
  onLoadReplay?: (file: ReplayFile) => void;
}) {
  const world = useStaticWorld(village.map);
  const memory = useRef(new BattleMemory());
  const onDecoded = useCallback((o: ObservationView) => memory.current.note(o), []);
  const sim = useSimSession({ scenario, seed, onDecoded, replay: replay?.replay });
  const { observation } = sim;
  const control = useUnitControl(replay ? null : sim.client, observation);
  const drawn = useRef<{ owners: (number | null)[]; enemies: (number | null)[] }>({
    owners: [],
    enemies: [],
  });
  const drawnAt = useRef(new Map<number, readonly [number, number, number]>());
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;
  const readouts = useRef<ReadoutLayerHandle>(null);

  useEffect(() => memory.current.clear(), [sim.client]);

  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface", "apart"),
    [world],
  );
  const fallenKey = (observation?.knownProps ?? [])
    .flatMap((p) => (p.replaces === null ? [] : [p.replaces]))
    .join();
  const standing = useMemo(
    () =>
      world &&
      buildStandingStructures(
        world.exports,
        world.layout,
        new Set(fallenKey ? fallenKey.split(",").map(Number) : []),
      ),
    [world, fallenKey],
  );
  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );
  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      const d = sideInstances("blue", poses, observation, selectedRef.current);
      drawn.current = d;
      drawnAt.current = new Map(poses.map((p) => [p.id, p.position]));
      return d.instances;
    },
    [observation, sim.interpolator],
  );
  const overlay = useMemo(
    () =>
      world && observation && standing
        ? buildBattleOverlay(observation, memory.current, control.selected, standing, surfaceZ)
        : undefined,
    [world, observation, standing, surfaceZ, control.selected],
  );
  const onPick = useCallback(
    (pick: LabPick) => {
      if (!world) return;
      const right = pick.button === "right";
      const ground = right ? groundUnderRay(world.view, pick.ray) : null;
      const k = pick.instance;
      control.onPointer({
        ...pick,
        unit: k >= 0 ? (drawn.current.owners[k] ?? null) : null,
        enemy: k >= 0 ? (drawn.current.enemies[k] ?? null) : null,
        building: right ? buildingUnderRay(world, pick.ray) : null,
        ground: ground && [ground[0], ground[1]],
      });
    },
    [world, control],
  );

  const exportReplay = useCallback(async () => {
    if (!sim.client) return null;
    const file: ReplayFile = { variant, replay: await sim.client.replay() };
    const text = JSON.stringify(file);
    try {
      localStorage.setItem(LAST_REPLAY_KEY, text);
    } catch {
      // storage unavailable: the download still works
    }
    const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `village-${variant}-seed${seed}-tick${observation?.tick ?? 0}.json`;
    a.click();
    // Revoking at once can cancel the download in some browsers.
    setTimeout(() => URL.revokeObjectURL(url), 10_000);
    return file;
  }, [sim.client, variant, seed, observation]);

  const paused = sim.status.status === "paused";
  const togglePause = () => (paused ? sim.client?.resume() : sim.client?.pause());

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    digest: (tick: number) => sim.digests.current.get(tick),
    error: () => sim.error,
    selected: () => control.selected,
    select: (ids: number[]) => control.setSelected(ids),
    acks: () => control.acks,
    exportReplay,
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
  };

  if (!meshes) return null;
  const enc = observation?.encounter;
  const elapsed = (observation?.tick ?? 0) / TICK_HZ;
  const minutes = Math.floor(elapsed / 60);
  const seconds = Math.floor(elapsed % 60);
  return (
    <>
      <LabViewport
        fixture={replay ? "village-replay" : "village"}
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={VILLAGE_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
        onFrame={(project, distance) => readouts.current?.place(project, distance, drawnAt.current)}
        diagnostics={diagnostics}
      />
      <ReadoutLayer observation={observation} selected={control.selected} handle={readouts} />
      <aside className="lab-panel" data-testid="village-panel">
        <strong>{replay ? "Village replay" : "Village battle"}</strong>
        <div data-testid="status">
          {VARIANT_LABEL[variant]} · seed {replay ? `${replaySeed(replay)} (saved battle)` : seed} ·{" "}
          {minutes}:{String(seconds).padStart(2, "0")} · {sim.status.status}
        </div>
        {sim.error && (
          <div className="lab-rejected" data-testid="error">
            {sim.error}
          </div>
        )}
        <div data-testid="encounter">
          Hold the village:{" "}
          {enc ? `${enc.heldS.toFixed(0)}/${HOLD_S} s held · ${RESULT_TEXT[enc.result]}` : "—"}
        </div>
        {!replay && setVariant && setSeed && (
          <div className="lab-row">
            <select
              value={variant}
              onChange={(e) => setVariant(e.target.value as Variant)}
              aria-label="Variant"
            >
              {(Object.keys(VARIANT_LABEL) as Variant[]).map((v) => (
                <option key={v} value={v}>
                  {VARIANT_LABEL[v]}
                </option>
              ))}
            </select>
            <label>
              Seed{" "}
              <input
                type="number"
                value={seed}
                style={{ width: 90 }}
                onChange={(e) => setSeed(Number(e.target.value) || 0)}
              />
            </label>
          </div>
        )}
        <div className="lab-row">
          <button type="button" onClick={togglePause}>
            {paused ? "Resume" : "Pause"}
          </button>
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
          {!replay && (
            <button type="button" onClick={() => void exportReplay()}>
              Save replay
            </button>
          )}
          {!replay && (
            <a className="lab-hint" href="/replay/village">
              Watch saved replay
            </a>
          )}
        </div>
        {replay && onLoadReplay && <ReplayImport onLoad={onLoadReplay} />}
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
        {!replay && (
          <>
            <div className="lab-hint">Commands, newest first</div>
            <ul className="lab-log" data-testid="ack-log">
              {control.acks.length === 0 && <li>None yet</li>}
              {control.acks.map((a) => (
                <AckLine key={a.seq} entry={a} />
              ))}
            </ul>
          </>
        )}
      </aside>
    </>
  );
}
