import { useState } from "react";
import village from "@fixtures/village.json";
import { durableSoldiers, VILLAGE_RULES } from "../scenarios";
import { SavedEncounter, villageScenario } from "../savedMaps";
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
/** The objective's readout once the battle is decided; while it runs, the
 *  hold's count. */
const RESULT_TEXT: Record<string, string> = {
  captured: "VILLAGE CAPTURED",
  defeated: "DEFEATED",
  inconclusive: "INCONCLUSIVE: PLAY ON",
};

const isVariant = (v: unknown): v is Variant => typeof v === "string" && v in VARIANT_LABEL;

/** The battle's seed: the scenario's own, or `?seed=` for testing. No
 *  player UI shows or sets it. */
function urlSeed(): number {
  const seed = Number(new URLSearchParams(window.location.search).get("seed"));
  return Number.isInteger(seed) && seed > 0 ? seed : village.seed;
}

/** Blue's comparison scripts (`village_report`'s names) a watched battle can
 *  play; `?script=<name>` picks one. */
const WATCH_SCRIPTS = ["scout-suppress-flank", "unsupported-road-push"] as const;
function watchedScript(fallback: string): string {
  const name = new URLSearchParams(window.location.search).get("script");
  return name && (WATCH_SCRIPTS as readonly string[]).includes(name) ? name : fallback;
}

/** The village scenario JSON for `variant`, built by the simulation. */
function useVillageScenario(variant: Variant): string | { error: string } | null {
  const built = useBuiltScenario(variant, villageScenario);
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

/** The pause menu's scenario: the variants as a list. A new variant
 *  restarts the battle. */
function ScenarioPicker({
  variant,
  setVariant,
}: {
  variant: Variant;
  setVariant: (v: Variant) => void;
}) {
  return (
    <section className="hud-menu-section" aria-label="Scenario">
      <div role="radiogroup" aria-label="Variant" className="hud-menu-list">
        {(Object.keys(VARIANT_LABEL) as Variant[]).map((v) => (
          <button
            key={v}
            type="button"
            role="radio"
            aria-checked={v === variant}
            className="hud-menu-choice"
            onClick={() => setVariant(v)}
          >
            {VARIANT_LABEL[v]}
          </button>
        ))}
      </div>
    </section>
  );
}

/** The battle's clock, from the published tick. */
function BattleClock({ tick }: { tick: number }) {
  const s = tick / TICK_HZ;
  return (
    <span className="hud-clock" data-testid="clock">
      {Math.floor(s / 60)}:{String(Math.floor(s % 60)).padStart(2, "0")}
    </span>
  );
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

/** The lean-out firefight on the village's ground, the village map's saved
 *  encounter `lean` (`fixtures/maps/village/encounters/lean.json`): a
 *  blue squad at rest just inside the west wood trades fire with a red
 *  squad in the open 45 m east; soldiers too tough to fall, so the fight
 *  holds. Men lean out from their trees, fire and tuck back in. */
const LEAN_RULES = durableSoldiers(VILLAGE_RULES);

/** /battle/village/lean: the lean-out firefight, watched. */
export function VillageLean() {
  return (
    <SavedEncounter map="village" encounter="lean" rules={LEAN_RULES}>
      {(battle) => (
        <BattleView
          fixture="village-lean"
          scenario={battle.scenario}
          seed={village.seed}
          camera={villageCamera.opening()}
          status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
        />
      )}
    </SavedEncounter>
  );
}

function VillageEncounter({ script }: { script: string | null }) {
  const [variant, setVariant] = useState<Variant>("ordinary");
  const [seed] = useState(urlSeed);
  const scenario = useVillageScenario(variant);
  if (!scenario) return null;
  if (typeof scenario !== "string") return <Failed error={scenario.error} />;
  return (
    <VillageView
      key={variant}
      scenario={scenario}
      seed={seed}
      variant={variant}
      setVariant={setVariant}
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
    <label className="hud-menu-file">
      Load a saved battle{" "}
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
      {error && <span className="hud-error"> {error}</span>}
    </label>
  );
}

function VillageView({
  scenario,
  seed,
  variant,
  setVariant,
  replay,
  onLoadReplay,
  script = null,
}: {
  scenario: string;
  seed: number;
  variant: Variant;
  setVariant?: (v: Variant) => void;
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

  // The top bar: the objective and the clock, nothing else.
  const status = ({ sim }: BattleSession) => {
    const enc = sim.observation?.encounter;
    return (
      <>
        <span className="hud-objective" data-testid="encounter">
          {!enc
            ? "—"
            : enc.result === "running"
              ? `HOLD ${enc.heldS.toFixed(0)}/${HOLD_S} s`
              : RESULT_TEXT[enc.result]}
        </span>
        <BattleClock tick={sim.observation?.tick ?? 0} />
      </>
    );
  };
  // The pause menu: the scenario (a replay's is fixed: its name) and the
  // replay files.
  const menu = (session: BattleSession) => (
    <>
      {replay || !setVariant ? (
        <div className="hud-menu-note" data-testid="status">
          {VARIANT_LABEL[variant]}
          {replay && " · saved battle"}
        </div>
      ) : (
        <ScenarioPicker variant={variant} setVariant={setVariant} />
      )}
      {!replay && (
        <button type="button" className="hud-menu-item" onClick={() => void exportReplay(session)}>
          Save replay
        </button>
      )}
      {!replay && (
        <a className="hud-menu-item" href="/replay/village">
          Watch saved replay
        </a>
      )}
      {replay && onLoadReplay && <ReplayImport onLoad={onLoadReplay} />}
    </>
  );

  return (
    <BattleView
      fixture={replay ? "village-replay" : "village"}
      scenario={scenario}
      seed={seed}
      replay={replay?.replay}
      scripted={scripted}
      camera={VILLAGE_CAMERA}
      status={status}
      menu={menu}
      diagnostics={(session) => ({ exportReplay: () => exportReplay(session) })}
    />
  );
}
