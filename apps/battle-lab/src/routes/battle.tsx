// /battle: a prepared battle, in the same battle view the village plays in.
// The address says which (`battleLinks.ts`): a generated map by type, size
// and seed (what the main menu starts), a saved map and its encounter, or
// the saved replay of either. A preparation worker resolves the map through
// the one map owner and lays the encounter on it; a loading screen covers
// the wait, and a refusal says which stage refused and never plays anything
// else in its place.
import { useEffect, useMemo, useRef, useState } from "react";
import config from "@fixtures/generated-battle.json";
import recipes from "@fixtures/encounters.json?raw";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { prepareBattle, PreparationFailed, type PreparedSession } from "@web/battle/prepare/client";
import type {
  PrepareBattleRequest,
  PrepareStage,
  RefusalStage,
} from "@web/battle/prepare/protocol";
import { listMaps } from "@web/maps/catalogue";
import { generationRequest, type MapChoice } from "@web/maps/source";
import { askedBattle, menuHref, type AskedBattle } from "../battleLinks";
import { BattleClock, objectiveStatus } from "../battleStatus";
import { BattleView, type BattleLoadStage } from "../BattleView";
import { LoadingScreen, type LoadingFailure, type LoadingStage } from "../LoadingScreen";
import {
  isPreparedReplay,
  readSavedReplay,
  PREPARED_REPLAY_ROUTE,
  ReplayImport,
  saveReplay,
  type PreparedReplayFile,
} from "../replayFile";
import { GAME_RULES } from "../scenarios";
import { gameCamera } from "../gameCamera";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import type { BattleSession } from "../useBattleSession";

const GENERATOR = { presets, templates };

/** The generated map a request asks for, if it asks for one. */
const generatedChoice = (request: PrepareBattleRequest): MapChoice | null =>
  request.map_source.kind === "generated" ? request.map_source.request : null;

/** What the loading screen and the top bar call the battle's map: a
 *  generated map's choice, or a saved map's listed name and its encounter. */
function subjectOf(request: PrepareBattleRequest): string {
  const source = request.map_source;
  if (source.kind === "generated")
    return `${source.request.type} · ${source.request.size}`.toUpperCase();
  const listed = listMaps().find((map) => map.id === source.id);
  return `${listed?.label ?? source.id} · ${request.recipe_id}`.toUpperCase();
}

type Stage = PrepareStage | "world" | "renderer";
const stagesOf = (request: PrepareBattleRequest): (LoadingStage & { id: Stage })[] => [
  {
    id: "map",
    label: request.map_source.kind === "generated" ? "Generating the map" : "Loading the map",
  },
  { id: "encounter", label: "Placing forces" },
  { id: "world", label: "Building the battlefield" },
  { id: "renderer", label: "Starting the battle" },
];

/** What the player is told when a stage refuses. The request stands as
 *  asked: nothing else is tried in its place. */
function failureOf(request: PrepareBattleRequest, error: unknown, replay: boolean): LoadingFailure {
  const stage: RefusalStage | null = error instanceof PreparationFailed ? error.stage : null;
  const details =
    error instanceof PreparationFailed && error.diagnostics.length
      ? error.diagnostics.map((d) => `${d.code} at ${d.location}: ${d.message}`)
      : [error instanceof Error ? error.message : String(error)];
  // A replay's request made a battle once. If it makes none now, the build
  // that reads it is not the build that saved it.
  if (replay && stage)
    return {
      message: "This replay cannot be played on this build.",
      advice: "It was saved by another version of the game, which made its map differently.",
      details,
    };
  const choice = generatedChoice(request);
  const message =
    stage === "request"
      ? "This battle cannot be requested."
      : stage === "map"
        ? choice
          ? "This map could not be built."
          : "This map could not be loaded."
        : stage === "encounter"
          ? "No battle could be placed on this map."
          : "The battle could not be prepared.";
  return {
    message,
    advice:
      choice && (stage === "map" || stage === "encounter")
        ? "Deploy again from the menu for another map."
        : undefined,
    details,
  };
}

/** When each stage of this page's loading finished, in milliseconds since
 *  navigation started (which is when the player pressed play). */
type StartupMarks = Partial<Record<"prepared" | BattleLoadStage, number>>;

function Refused({ subject, failure }: { subject: string; failure: LoadingFailure }) {
  return (
    <LoadingScreen title="Deploying" subject={subject} stages={[]} current="" failure={failure} />
  );
}

/** /battle: the battle its address asks for. */
export default function Battle() {
  const [asked] = useState(() => askedBattle(window.location.search));
  if ("error" in asked)
    return (
      <Refused
        subject="UNKNOWN BATTLE"
        failure={{
          message: "This link does not name a battle.",
          advice: "Start one from the menu.",
          details: [asked.error],
        }}
      />
    );
  return asked.kind === "replay" ? <SavedReplay /> : <AskedBattleView asked={asked} />;
}

/** The request for what the address asks, pinned to this build's generator,
 *  presets and catalogue. */
function AskedBattleView({ asked }: { asked: Exclude<AskedBattle, { kind: "replay" }> }) {
  const request = useBuiltScenario(asked, (wasm, a): PrepareBattleRequest => {
    const rest = {
      recipe_id: a.recipe,
      encounter_seed: a.encounterSeed,
      battle_seed: a.battleSeed,
    };
    return a.kind === "catalogue"
      ? { map_source: { kind: "catalogue", id: a.id }, ...rest }
      : {
          map_source: {
            kind: "generated",
            request: generationRequest(wasm, a.map, GENERATOR, config.limits),
          },
          ...rest,
        };
  });
  if (!request) return null;
  if (buildFailed(request))
    return (
      <Refused
        subject="UNKNOWN BATTLE"
        failure={{ message: "The battle could not be prepared.", details: [request.error] }}
      />
    );
  return <PreparedBattleView request={request} />;
}

/** /battle?replay=saved: watch the saved battle; input is off. */
function SavedReplay() {
  // Each loaded file gets a fresh view, even one identical to the last.
  const [loaded, setLoaded] = useState<{ file: PreparedReplayFile | null; n: number }>(() => {
    const file = readSavedReplay();
    return { file: file && isPreparedReplay(file) ? file : null, n: 0 };
  });
  const setFile = (file: PreparedReplayFile) => setLoaded((l) => ({ file, n: l.n + 1 }));
  if (!loaded.file)
    return (
      <main className="menu">
        <div className="hud-panel menu-body">
          <h1>Replay</h1>
          <ReplayImport plays={isPreparedReplay} onLoad={setFile} />
          <a className="hud-menu-item" href="/">
            Main menu
          </a>
        </div>
      </main>
    );
  return (
    <PreparedBattleView
      key={loaded.n}
      request={loaded.file.request}
      replay={loaded.file}
      onLoadReplay={setFile}
    />
  );
}

function PreparedBattleView({
  request,
  replay,
  onLoadReplay,
}: {
  request: PrepareBattleRequest;
  /** A saved battle on this request, to watch. */
  replay?: PreparedReplayFile;
  onLoadReplay?: (file: PreparedReplayFile) => void;
}) {
  const [stage, setStage] = useState<Stage>("map");
  const [prepared, setPrepared] = useState<PreparedSession | null>(null);
  const [failure, setFailure] = useState<LoadingFailure | null>(null);
  const marks = useRef<StartupMarks>({});

  // One request, one worker. Leaving (or another request) closes it, and a
  // closed request's answer is never delivered.
  useEffect(() => {
    setPrepared(null);
    setFailure(null);
    setStage("map");
    marks.current = {};
    const preparation = prepareBattle(
      {
        type: "prepare",
        request,
        documents: { rules: JSON.stringify(GAME_RULES), presets, templates, recipes },
      },
      setStage,
    );
    preparation.battle.then(
      (battle) => {
        marks.current.prepared = performance.now();
        setStage("world");
        setPrepared(battle);
      },
      (error: unknown) => setFailure(failureOf(request, error, !!replay)),
    );
    return () => preparation.cancel();
  }, [request, replay]);

  const subject = subjectOf(request);
  const cover = (
    <LoadingScreen
      title={replay ? "Loading replay" : "Deploying"}
      subject={subject}
      stages={stagesOf(request)}
      current={stage}
      failure={failure}
      back={menuHref(generatedChoice(request))}
    />
  );
  const view = useMemo(
    () =>
      prepared && {
        camera: {
          ...gameCamera.opening(),
          target: [prepared.report.start.at[0], prepared.report.start.at[1], 0] as [
            number,
            number,
            number,
          ],
        },
        cameraConfig: gameCamera.forMap(prepared.report.size, prepared.report.extents.rendered),
      },
    [prepared],
  );
  if (!prepared || !view) return cover;

  const exportReplay = async ({ sim }: BattleSession) => {
    if (!sim.client) return null;
    const file: PreparedReplayFile = { request, replay: await sim.client.replay() };
    const name = subject.toLowerCase().replaceAll(" · ", "-").replaceAll(" ", "");
    saveReplay(file, `battle-${name}-tick${sim.latest.current?.tick ?? 0}.json`);
    return file;
  };
  const objective = prepared.report.objective;
  return (
    <BattleView
      fixture="generated"
      prepared={prepared}
      scenario={prepared.scenario}
      seed={request.battle_seed}
      replay={replay?.replay}
      camera={view.camera}
      cameraConfig={view.cameraConfig}
      cover={cover}
      onLoadStage={(loaded) => {
        marks.current[loaded] ??= performance.now();
        if (loaded === "world") setStage("renderer");
      }}
      status={
        objective
          ? objectiveStatus(objective.hold_s, "OBJECTIVE CAPTURED")
          : ({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />
      }
      menu={(session) => (
        <>
          <div className="hud-menu-note" data-testid="status">
            {subject}
            {replay && " · saved battle"}
          </div>
          {!replay && (
            <button
              type="button"
              className="hud-menu-item"
              onClick={() => void exportReplay(session)}
            >
              Save replay
            </button>
          )}
          {!replay && (
            <a className="hud-menu-item" href={PREPARED_REPLAY_ROUTE}>
              Watch saved replay
            </a>
          )}
          {replay && onLoadReplay && (
            <ReplayImport plays={isPreparedReplay} onLoad={onLoadReplay} />
          )}
        </>
      )}
      diagnostics={(session) => ({
        /** What the preparation worker made: the request, the map's identity
         *  and counts, the planned encounter and what each stage cost. */
        prepared: () => prepared.report,
        /** When each loading stage finished, ms since navigation started. */
        startup: () => ({ ...marks.current }),
        exportReplay: () => exportReplay(session),
      })}
    />
  );
}
