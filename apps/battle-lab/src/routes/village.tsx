import { useMemo, useState } from "react";
import village from "@fixtures/village.json";
import { VILLAGE_RULES } from "../scenarios";
import { BattleView } from "../BattleView";
import { useBuiltScenario } from "../useBuiltScenario";
import { villageCamera } from "../villageCamera";
import type { BattleSession } from "../useBattleSession";
import type { ScriptedSim } from "../useSimSession";

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

const VILLAGE_CAMERA = villageCamera.opening();

const HOLD_S = village.encounter.hold_s;
const TICK_HZ = village.tick_hz;
const RESULT_TEXT: Record<string, string> = {
  running: "in progress",
  captured: "village captured",
  defeated: "blue has no combat units left",
  inconclusive: `${village.encounter.max_assessment_s / 60} minutes passed: inconclusive (play on)`,
};

const isVariant = (v: unknown): v is Variant => typeof v === "string" && v in VARIANT_LABEL;

/** Blue's comparison scripts (`village_report`'s names) a watched battle can
 *  play; `?script=<name>` picks one. */
const WATCH_SCRIPTS = ["scout-suppress-flank", "unsupported-road-push"] as const;
function watchedScript(fallback: string): string {
  const name = new URLSearchParams(window.location.search).get("script");
  return name && (WATCH_SCRIPTS as readonly string[]).includes(name) ? name : fallback;
}

/** The village scenario JSON for `variant`, built by the simulation. */
function useVillageScenario(variant: Variant): string | { error: string } | null {
  const built = useBuiltScenario(variant, (wasm, v) =>
    wasm.village_scenario(JSON.stringify(VILLAGE_RULES), v),
  );
  return built && typeof built !== "string"
    ? { error: `the village scenario could not be built: ${built.error}` }
    : built;
}

function Failed({ error }: { error: string }) {
  return (
    <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
      {error}
    </main>
  );
}

/** The battle's scenario as a top-bar readout ("Ordinary ambush · seed 42")
 *  that opens a picker: the variants as a list, the seed with a field and
 *  a step either way. A new variant or seed restarts the battle. */
function ScenarioPicker({
  variant,
  seed,
  setVariant,
  setSeed,
}: {
  variant: Variant;
  seed: number;
  setVariant: (v: Variant) => void;
  setSeed: (s: number) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <span
      className="hud-scenario"
      onKeyDown={(e) => {
        if (e.key === "Escape") setOpen(false);
      }}
    >
      <button
        type="button"
        className="hud-scenario-readout"
        aria-label="Scenario"
        aria-expanded={open}
        aria-haspopup="dialog"
        onClick={() => setOpen(!open)}
      >
        {VARIANT_LABEL[variant]} · seed {seed}{" "}
        <span className="hud-caret" aria-hidden="true">
          ▾
        </span>
      </button>
      {open && (
        <div
          className="lab-panel hud-menu"
          role="dialog"
          aria-label="Choose scenario"
          data-occludes-readouts
        >
          <div className="hud-menu-title">Scenario</div>
          <div role="radiogroup" aria-label="Variant" className="hud-menu-list">
            {(Object.keys(VARIANT_LABEL) as Variant[]).map((v) => (
              <button
                key={v}
                type="button"
                role="radio"
                aria-checked={v === variant}
                className="hud-menu-item"
                onClick={() => setVariant(v)}
              >
                {VARIANT_LABEL[v]}
              </button>
            ))}
          </div>
          <div className="hud-menu-seed">
            <button type="button" aria-label="Previous seed" onClick={() => setSeed(seed - 1)}>
              ◂
            </button>
            <label>
              Seed{" "}
              <input
                type="number"
                value={seed}
                onChange={(e) => setSeed(Number(e.target.value) || 0)}
              />
            </label>
            <button type="button" aria-label="Next seed" onClick={() => setSeed(seed + 1)}>
              ▸
            </button>
          </div>
        </div>
      )}
    </span>
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
  return <VillageEncounter script={null} />;
}

/** /battle/village/watch: blue is played by a comparison script (default
 *  `scout-suppress-flank`) and the player watches with a free camera. */
export function VillageWatch() {
  const [script] = useState(() => watchedScript(WATCH_SCRIPTS[0]));
  return <VillageEncounter script={script} />;
}

/** The lean-out firefight (battle-look slice 27d) on the village's ground: a
 *  blue squad at rest just inside the west wood trades fire with a red
 *  squad in the open 45 m east; soldiers too tough to fall, so the fight
 *  holds. Men lean out from their trees, fire and tuck back in. */
const LEAN_UNITS = [
  { side: "blue", kind: "rifle", position: [866, 962], yaw: 0 },
  { side: "red", kind: "rifle", position: [912, 966], yaw: Math.PI },
];

function leanScenario(scenario: string): string {
  const s = JSON.parse(scenario) as Record<string, unknown> & {
    rules: { catalog: { soldiers?: Record<string, { hp: number }> }[] };
  };
  // Every soldier kind of the (resolved) catalog all but unkillable.
  for (const doc of s.rules.catalog)
    for (const kind of Object.values(doc.soldiers ?? {})) kind.hp = 1.0e6;
  return JSON.stringify({ ...s, units: LEAN_UNITS, scripts: [], opponent: null, encounter: null });
}

/** /battle/village/lean: the lean-out firefight, watched. */
export function VillageLean() {
  const scenario = useVillageScenario("ordinary");
  const lean = useMemo(
    () => (typeof scenario === "string" ? leanScenario(scenario) : null),
    [scenario],
  );
  if (!scenario) return null;
  if (typeof scenario !== "string") return <Failed error={scenario.error} />;
  if (!lean) return null;
  return (
    <BattleView
      fixture="village-lean"
      scenario={lean}
      seed={village.seed}
      camera={villageCamera.opening()}
      title="Lean-out firefight"
      panel={({ sim }) => (
        <div data-testid="status">
          A squad in the wood leans out to fire · tick {sim.observation?.tick ?? 0} ·{" "}
          {sim.status.status}
        </div>
      )}
    />
  );
}

function VillageEncounter({ script }: { script: string | null }) {
  const [variant, setVariant] = useState<Variant>("ordinary");
  const [seed, setSeed] = useState<number>(village.seed);
  const scenario = useVillageScenario(variant);
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
      script={script}
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
  const scenario = useVillageScenario(file?.variant ?? "ordinary");
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
  script = null,
}: {
  scenario: string;
  seed: number;
  variant: Variant;
  setVariant?: (v: Variant) => void;
  setSeed?: (s: number) => void;
  replay?: ReplayFile;
  onLoadReplay?: (file: ReplayFile) => void;
  /** Blue's script when watching (`?script=`), else blue is the player's. */
  script?: string | null;
}) {
  const [scripted] = useState<ScriptedSim | undefined>(() =>
    script ? { script, warmTo: 0, onWarm: () => {}, onTick: () => {} } : undefined,
  );
  const exportReplay = async ({ sim }: BattleSession) => {
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
    a.download = `village-${variant}-seed${seed}-tick${sim.latest.current?.tick ?? 0}.json`;
    a.click();
    // Revoking at once can cancel the download in some browsers.
    setTimeout(() => URL.revokeObjectURL(url), 10_000);
    return file;
  };

  const panel = (session: BattleSession) => {
    const { sim } = session;
    const tick = sim.observation?.tick ?? 0;
    const enc = sim.observation?.encounter;
    const elapsed = tick / TICK_HZ;
    const clock = `${Math.floor(elapsed / 60)}:${String(Math.floor(elapsed % 60)).padStart(2, "0")}`;
    const paused = sim.status.status === "paused";
    return (
      <>
        <div data-testid="status">
          {!replay && setVariant && setSeed ? (
            <ScenarioPicker
              variant={variant}
              seed={seed}
              setVariant={setVariant}
              setSeed={setSeed}
            />
          ) : (
            `${VARIANT_LABEL[variant]} · seed ${replay ? `${replaySeed(replay)} (saved battle)` : seed}`
          )}{" "}
          · {clock} · {sim.status.status}
          {scripted && ` · blue: ${scripted.script} (watching)`}
        </div>
        <div data-testid="encounter">
          Hold the village:{" "}
          {enc ? `${enc.heldS.toFixed(0)}/${HOLD_S} s held · ${RESULT_TEXT[enc.result]}` : "—"}
        </div>
        <div className="lab-row">
          <button
            type="button"
            onClick={() => (paused ? sim.client?.resume() : sim.client?.pause())}
          >
            {paused ? "Resume" : "Pause"}
          </button>
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
          {!replay && (
            <button type="button" onClick={() => void exportReplay(session)}>
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
      </>
    );
  };

  return (
    <BattleView
      fixture={replay ? "village-replay" : "village"}
      scenario={scenario}
      seed={seed}
      replay={replay?.replay}
      scripted={scripted}
      camera={VILLAGE_CAMERA}
      title={replay ? "Village replay" : "Village battle"}
      panel={panel}
      diagnostics={(session) => ({ exportReplay: () => exportReplay(session) })}
    />
  );
}
