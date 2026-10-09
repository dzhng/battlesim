import { Link } from "react-router";
import { usePageVisitActive, usePublishBattleAddress } from "../navigation";
// /battle: a prepared battle, in the battle view every battle plays in.
// Exact addresses retain one requested battle. Ordinary Play selects an admitted
// preparation before publishing its exact address and starting its worker.
import { useEffect, useMemo, useRef, useState } from "react";
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { prepareBattle, PreparationFailed, type PreparedSession } from "@web/battle/prepare/client";
import { admitBattle, type Admission } from "@web/battle/prepare/admission";
import type {
  PrepareBattleRequest,
  PrepareStage,
  RefusalStage,
} from "@web/battle/prepare/protocol";
import { generationRequest, type MapChoice } from "@web/maps/source";
import {
  askedBattle,
  menuHref,
  preparedBattleHref,
  spoken,
  type AskedBattle,
} from "../battleLinks";
import { BattleClock } from "../battleStatus";
import { BattleView, type BattleLoadStage } from "../BattleView";
import { LoadingScreen, type LoadingFailure, type LoadingStage } from "../LoadingScreen";
import {
  useSavedReplay,
  REPLAY_ROUTE,
  ReplayImport,
  ReplayLoading,
  saveReplay,
  type ReplayFile,
} from "../replayFile";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { gameCamera } from "../gameCamera";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import type { BattleSession } from "../useBattleSession";

const GENERATOR = { presets, templates };

/** The generated map a request asks for. */
const generatedChoice = (request: PrepareBattleRequest): MapChoice => request.map_source.request;

/** What the loading screen and the top bar call the battle's map: its
 *  generated map's choice. */
function subjectOf(request: PrepareBattleRequest): string {
  const { type, size, region } = request.map_source.request;
  return [type, size, region && spoken(region)].filter(Boolean).join(" · ").toUpperCase();
}

type Stage = PrepareStage | "world" | "renderer";
const STAGES: (LoadingStage & { id: Stage })[] = [
  { id: "map", label: "Generating the map" },
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
  const message =
    stage === "request"
      ? "This battle cannot be requested."
      : stage === "map"
        ? "This map could not be built."
        : stage === "encounter"
          ? "No battle could be placed on this map."
          : "The battle could not be prepared.";
  return {
    message,
    advice:
      stage === "map" || stage === "encounter"
        ? "This exact seed remains in the link. Start a new battle from the main menu."
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
  const request = useBuiltScenario(
    asked,
    (wasm, a): PrepareBattleRequest => ({
      map_source: {
        kind: "generated",
        request: generationRequest(
          wasm,
          { ...a.map, profile: "skirmish", seed: a.kind === "play" ? "0" : a.map.seed },
          GENERATOR,
          config.limits,
        ),
      },
      factions: a.factions,
      battle_seed: a.battleSeed,
    }),
  );
  if (!request) return null;
  if (buildFailed(request))
    return (
      <Refused
        subject="UNKNOWN BATTLE"
        failure={{ message: "The battle could not be prepared.", details: [request.error] }}
      />
    );
  return <PreparedBattleView request={request} play={asked.kind === "play"} />;
}

/** /battle?replay=saved: watch the saved battle; input is off. */
function SavedReplay() {
  const [loaded, setFile] = useSavedReplay();
  if (loaded.file === undefined) return <ReplayLoading />;
  const file = loaded.file;
  if (!file)
    return (
      <main className="menu">
        <div className="hud-panel menu-body">
          <h1>Replay</h1>
          <ReplayImport onLoad={setFile} />
          <Link className="hud-menu-item" to="/">
            Main menu
          </Link>
        </div>
      </main>
    );
  return (
    <PreparedBattleView
      key={loaded.n}
      request={file.battle.report.request}
      replay={file}
      onLoadReplay={setFile}
    />
  );
}

function PreparedBattleView({
  request,
  replay,
  play = false,
  onLoadReplay,
}: {
  request: PrepareBattleRequest;
  play?: boolean;
  /** A saved battle on this request, to watch. */
  replay?: ReplayFile;
  onLoadReplay?: (file: ReplayFile) => void;
}) {
  const publishAddress = usePublishBattleAddress();
  const visitActive = usePageVisitActive();
  const { rules } = useSessionCatalog();
  const [stage, setStage] = useState<Stage>("map");
  const [prepared, setPrepared] = useState<PreparedSession | null>(null);
  const [loadingRequest, setLoadingRequest] = useState(request);
  const [failure, setFailure] = useState<LoadingFailure | null>(null);
  const marks = useRef<StartupMarks>({});
  const admission = useRef<Admission | null>(null);

  // Preparation owns the candidate workers and, after admission, the winner.
  useEffect(() => {
    let live = true;
    setPrepared(null);
    setLoadingRequest(request);
    setFailure(null);
    setStage("map");
    marks.current = {};
    const documents = { rules: JSON.stringify(rules), presets, templates };
    const selection = play
      ? admitBattle(
          {
            candidate: (seed) => ({
              ...request,
              map_source: { kind: "generated", request: { ...request.map_source.request, seed } },
            }),
            documents,
            policy: config.admission,
            onRequest: setLoadingRequest,
          },
          setStage,
        )
      : null;
    const preparation =
      selection ??
      prepareBattle(
        replay
          ? { type: "prepare-replay", battle: replay.battle }
          : { type: "prepare", request, documents },
        setStage,
      );
    admission.current = selection;
    preparation.battle.then(
      (battle) => {
        if (!live || !visitActive()) return;
        if (play) publishAddress(preparedBattleHref(battle.report.request));
        marks.current.prepared = performance.now();
        setStage("world");
        setPrepared(battle);
      },
      (error: unknown) => {
        if (!live || !visitActive()) return;
        setFailure(
          play
            ? {
                message: "The battle could not be prepared.",
                advice: "Return to the menu and try again.",
                details: admission.current?.attempts.flatMap((a) =>
                  a.failure?.diagnostics.length
                    ? a.failure.diagnostics.map((d) => `${d.code} at ${d.location}: ${d.message}`)
                    : [a.failure?.message ?? a.outcome],
                ) ?? [String(error)],
              }
            : failureOf(request, error, !!replay),
        );
      },
    );
    return () => {
      live = false;
      preparation.cancel();
    };
  }, [request, replay, play, publishAddress, visitActive, rules]);

  const activeRequest = prepared?.report.request ?? loadingRequest;
  const subject = subjectOf(activeRequest);
  const cover = (
    <LoadingScreen
      title={replay ? "Loading replay" : "Deploying"}
      subject={subject}
      stages={STAGES}
      current={stage}
      failure={failure}
      back={menuHref(
        play && !prepared
          ? (({ type, size, region }) => ({ type, size, region }))(request.map_source.request)
          : generatedChoice(activeRequest),
      )}
    />
  );
  const view = useMemo(
    () =>
      prepared && {
        camera: {
          ...gameCamera.fromStart(prepared.report.start.at, prepared.report.size),
        },
        cameraConfig: gameCamera.forMap(prepared.report.size, prepared.report.extents.rendered),
      },
    [prepared],
  );
  if (!prepared || !view) return cover;

  const exportReplay = async ({ sim }: BattleSession) => {
    if (!sim.client) return null;
    const file: ReplayFile = {
      battle: { scenario: prepared.scenario, report: prepared.report },
      replay: await sim.client.replay(),
    };
    const name = subject.toLowerCase().replaceAll(" · ", "-").replaceAll(" ", "");
    await saveReplay(file, `battle-${name}-tick${sim.latest.current?.tick ?? 0}.json`);
    return file;
  };
  return (
    <BattleView
      fixture="generated"
      prepared={prepared}
      scenario={prepared.scenario}
      seed={activeRequest.battle_seed}
      replay={replay?.replay}
      camera={view.camera}
      cameraConfig={view.cameraConfig}
      spawn={prepared.report.start.at}
      cover={cover}
      onLoadStage={(loaded) => {
        marks.current[loaded] ??= performance.now();
        if (loaded === "world") setStage("renderer");
      }}
      status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
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
            <Link className="hud-menu-item" to={REPLAY_ROUTE}>
              Watch saved replay
            </Link>
          )}
          {replay && onLoadReplay && <ReplayImport onLoad={onLoadReplay} />}
        </>
      )}
      diagnostics={(session) => ({
        /** What the preparation worker made: the request, the map's identity
         *  and counts, where blue starts and what each stage cost. */
        prepared: () => prepared.report,
        /** When each loading stage finished, ms since navigation started. */
        startup: () => ({ ...marks.current }),
        exportReplay: () => exportReplay(session),
      })}
    />
  );
}
