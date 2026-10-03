import { useState } from "react";
import game from "@fixtures/game.json";
import { restartBattle } from "@web/mechanicsLifecycle";
import { durableSoldiers, GAME_RULES } from "../scenarios";
import { SavedEncounter, villageScenario } from "../savedMaps";
import { BattleView } from "../BattleView";
import { useBuiltScenario } from "../useBuiltScenario";
import { gameCamera } from "../gameCamera";
import { BattleClock, objectiveStatus } from "../battleStatus";
import {
  isPreparedReplay,
  useSavedReplay,
  ReplayImport,
  ReplayLoading,
  saveReplay,
  type ReplayFile as SavedFile,
} from "../replayFile";
import type { BattleSession } from "../useBattleSession";
import type { ScriptedSim } from "../useSimSession";

type Variant = "ordinary" | "prepared_crossfire";
const VARIANT_LABEL: Record<Variant, string> = {
  ordinary: "Ordinary ambush",
  prepared_crossfire: "Prepared crossfire",
};
/** A saved village battle: the variant it was played on and every accepted
 *  command. */
interface ReplayFile {
  variant: Variant;
  replay: string;
}

const VILLAGE_CAMERA = gameCamera.opening();

const isVariant = (v: unknown): v is Variant => typeof v === "string" && v in VARIANT_LABEL;
/** Whether a saved battle is the village's (a prepared battle's has its own
 *  viewer). */
const isVillageReplay = (file: SavedFile): file is ReplayFile =>
  !isPreparedReplay(file) && isVariant(file.variant);

/** The battle's seed: the scenario's own, or `?seed=` for testing. No
 *  player UI shows or sets it. */
function urlSeed(): number {
  const seed = Number(new URLSearchParams(window.location.search).get("seed"));
  return Number.isInteger(seed) && seed > 0 ? seed : game.seed;
}

/** Blue's comparison scripts (`village_report`'s names) a watched battle can
 *  play; `?script=<name>` picks one. */
const WATCH_SCRIPTS = ["scout-suppress-flank", "unsupported-road-push"] as const;
function watchedScript(fallback: string): string {
  const name = new URLSearchParams(window.location.search).get("script");
  return name && (WATCH_SCRIPTS as readonly string[]).includes(name) ? name : fallback;
}

/** The village scenario JSON for `variant`, built by the simulation. */
function useVillageScenario(fixture: string, variant: Variant): string | { error: string } | null {
  const built = useBuiltScenario({ fixture, variant }, (wasm, o) =>
    villageScenario(wasm, o.fixture, o.variant),
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

/** /battle/village: play the encounter. */
export default function VillageBattle() {
  return <VillageEncounter fixture="village" script={null} />;
}

/** /battle/village/watch: blue is played by a comparison script (default
 *  `scout-suppress-flank`) and the player watches with a free camera. */
export function VillageWatch() {
  const [script] = useState(() => watchedScript(WATCH_SCRIPTS[0]));
  return <VillageEncounter fixture="village-watch" script={script} />;
}

/** The lean-out firefight on the village's ground, the village map's saved
 *  encounter `lean` (`fixtures/maps/village/encounters/lean.json`): a
 *  blue squad at rest just inside the west wood trades fire with a red
 *  squad in the open 45 m east; soldiers too tough to fall, so the fight
 *  holds. Men lean out from their trees, fire and tuck back in. */
const LEAN_RULES = durableSoldiers(GAME_RULES);

/** /battle/village/lean: the lean-out firefight, watched. */
export function VillageLean() {
  return (
    <SavedEncounter fixture="village-lean" encounter="lean" rules={LEAN_RULES}>
      {(battle) => (
        <BattleView
          fixture="village-lean"
          scenario={battle.scenario}
          seed={game.seed}
          camera={gameCamera.opening()}
          status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
        />
      )}
    </SavedEncounter>
  );
}

function VillageEncounter({ fixture, script }: { fixture: string; script: string | null }) {
  const [variant, setVariant] = useState<Variant>(() => {
    const requested = new URLSearchParams(window.location.search).get("variant");
    return isVariant(requested) ? requested : "ordinary";
  });
  const chooseVariant = (next: Variant) => {
    if (next !== variant) restartBattle(() => setVariant(next), { variant: next });
  };
  const [seed] = useState(urlSeed);
  const scenario = useVillageScenario(fixture, variant);
  if (!scenario) return null;
  if (typeof scenario !== "string") return <Failed error={scenario.error} />;
  return (
    <VillageView
      key={variant}
      scenario={scenario}
      seed={seed}
      variant={variant}
      setVariant={chooseVariant}
      script={script}
    />
  );
}

/** /replay/village: watch a saved battle; input is off, the defender is off. */
export function VillageReplay() {
  const [loaded, loadFile] = useSavedReplay();
  const setFile = (file: ReplayFile) => restartBattle(() => loadFile(file));
  if (loaded.file === undefined) return <ReplayLoading />;
  const file = loaded.file && isVillageReplay(loaded.file) ? loaded.file : null;
  if (!file)
    return (
      <main style={{ padding: 24 }}>
        <h1>Village replay</h1>
        <ReplayImport plays={isVillageReplay} onLoad={setFile} />
      </main>
    );
  return <LoadedVillageReplay key={loaded.n} file={file} onLoad={setFile} />;
}

function LoadedVillageReplay({
  file,
  onLoad,
}: {
  file: ReplayFile;
  onLoad: (file: ReplayFile) => void;
}) {
  const scenario = useVillageScenario("village-replay", file.variant);
  if (!scenario) return null;
  if (typeof scenario !== "string") return <Failed error={scenario.error} />;
  return (
    <VillageView
      scenario={scenario}
      seed={0}
      variant={file.variant}
      replay={file}
      onLoadReplay={onLoad}
    />
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
    await saveReplay(
      file,
      `village-${variant}-seed${seed}-tick${sim.latest.current?.tick ?? 0}.json`,
    );
    return file;
  };

  // The top bar: the objective and the clock, nothing else.
  const status = objectiveStatus(game.encounter.hold_s, "VILLAGE CAPTURED");
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
      {replay && onLoadReplay && <ReplayImport plays={isVillageReplay} onLoad={onLoadReplay} />}
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
