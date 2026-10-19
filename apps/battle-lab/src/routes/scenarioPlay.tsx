import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import { Link } from "react-router";
import { SessionCatalogProvider } from "@web/battle/catalog/context";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import { BattleView } from "../BattleView";
import { BattleClock } from "../battleStatus";
import { gameCamera } from "../gameCamera";
import { LoadingScreen } from "../LoadingScreen";
import { loadWasm } from "@web/battle/sim/module";
import {
  prepareUploadedScenarioBundle,
  scenarioBundleCatalog,
  type PreparedScenarioBundleSession,
} from "@web/battle/prepare/client";
import { readScenarioBundle } from "@web/battle/scenarioBundle";
import { useAppAudio } from "../AppAudio";

export default function ScenarioPlay() {
  const appAudio = useAppAudio();
  const [bundle, setBundle] = useState<string | null>(null);
  const [session, setSession] = useState<PreparedScenarioBundleSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stage, setStage] = useState<"bundle" | "world">("bundle");
  const [deploy, setDeploy] = useState(false);
  const [activeHuman, setActiveHuman] = useState<string | null | undefined>(undefined);
  const [humanCapable, setHumanCapable] = useState<{ id: string; side: string }[]>([]);
  const [replayText, setReplayText] = useState<string | undefined>();
  const playedAsset = useRef<string | null>(null);
  useEffect(() => {
    if (!bundle || !deploy) return;
    let live = true;
    let preparation: ReturnType<typeof prepareUploadedScenarioBundle> | null = null;
    void loadWasm().then(() => {
      if (!live) return;
      preparation = prepareUploadedScenarioBundle(
        bundle,
        (next) => {
          if (next === "bundle") setStage("bundle");
        },
        activeHuman,
      );
      preparation.battle.then(
        (loaded) => {
          if (!live) return;
          setStage("world");
          setSession(loaded);
        },
        (reason: unknown) =>
          live && setError(reason instanceof Error ? reason.message : String(reason)),
      );
    });
    return () => {
      live = false;
      preparation?.cancel();
    };
  }, [bundle, deploy, activeHuman]);

  async function upload(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setSession(null);
    setError(null);
    setStage("bundle");
    setDeploy(false);
    playedAsset.current = null;
    try {
      const text = await readScenarioBundle(file);
      const parsed = JSON.parse(text) as {
        scenario?: { players?: { id: string; side: string; human_capable?: boolean }[] };
      };
      const players = (parsed.scenario?.players ?? [])
        .filter((player) => player.human_capable)
        .map(({ id, side }) => ({ id, side }));
      setHumanCapable(players);
      setActiveHuman(players[0]?.id);
      setBundle(text);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  const report = session?.report ?? null;
  const prepared = useMemo(() => session?.session ?? null, [session]);

  if (!session && bundle && !deploy)
    return (
      <main className="menu scenario-play-menu">
        <div className="hud-panel menu-body scenario-play-upload">
          <h1>Choose controller</h1>
          <p>Choose at most one human-capable player for this local play session.</p>
          <label>
            Active human player
            <select
              value={activeHuman ?? "__none"}
              onChange={(event) =>
                setActiveHuman(event.target.value === "__none" ? null : event.target.value)
              }
            >
              <option value="__none">No human · AI only</option>
              {humanCapable.map((player) => (
                <option key={player.id} value={player.id}>
                  {player.id} · {player.side}
                </option>
              ))}
            </select>
          </label>
          <button type="button" className="hud-menu-item" onClick={() => setDeploy(true)}>
            Deploy scenario
          </button>
          <Link className="hud-menu-item" to="/labs/scenario?mode=play">
            Choose another package
          </Link>
        </div>
      </main>
    );

  if (!session && bundle && deploy)
    return (
      <LoadingScreen
        title="Deploying"
        subject="UPLOADED SCENARIO"
        stages={[
          { id: "bundle", label: "Validate package" },
          { id: "world", label: "Build world" },
        ]}
        current={stage}
        failure={
          error
            ? {
                message: error,
                advice: "Upload a canonical scenario package exported by the builder.",
                details: [error],
              }
            : null
        }
        back="/labs/scenario?mode=play"
      />
    );

  if (!session)
    return (
      <main className="menu scenario-play-menu">
        <div className="hud-panel menu-body scenario-play-upload">
          <h1>Play scenario</h1>
          <p>
            Upload a downloaded scenario package. The normal battle worker admits it without map
            generation.
          </p>
          <label className="hud-menu-file">
            <strong className="scenario-upload-title">Upload scenario</strong>
            <input type="file" accept="application/json,.json" onChange={upload} hidden />
            <span className="scenario-upload-copy">
              Choose a downloaded .json package to deploy it through the normal battle worker.
            </span>
          </label>
          {error && <p role="alert">{error}</p>}
          <Link className="hud-menu-item" to="/labs/scenario">
            Open scenario builder
          </Link>
          <Link className="hud-menu-item" to="/">
            Main menu
          </Link>
        </div>
      </main>
    );

  if (!report || !prepared) return null;
  return (
    <ScenarioBattle
      appAudio={appAudio}
      report={report}
      prepared={prepared}
      stage={stage}
      setError={setError}
      playedAsset={playedAsset}
      replayText={replayText}
      setReplayText={setReplayText}
    />
  );
}

function ScenarioBattle({
  appAudio,
  report,
  prepared,
  stage,
  setError,
  playedAsset,
  replayText,
  setReplayText,
}: {
  appAudio: ReturnType<typeof useAppAudio>;
  report: NonNullable<PreparedScenarioBundleSession["report"]>;
  prepared: NonNullable<PreparedScenarioBundleSession["session"]>;
  stage: "bundle" | "world";
  setError: (error: string) => void;
  playedAsset: { current: string | null };
  replayText?: string;
  setReplayText: (text: string) => void;
}) {
  const [catalog, setCatalog] = useState<SessionCatalog | null>(null);
  // Once AI takes the human player over, the simulation refuses this client's
  // orders for it, so input stops too.
  const [takenOver, setTakenOver] = useState(false);
  useEffect(() => {
    let live = true;
    void loadWasm()
      .then((wasm) => {
        if (live) setCatalog(scenarioBundleCatalog(prepared.scenario, wasm));
      })
      .catch((reason: unknown) => {
        if (live) setError(reason instanceof Error ? reason.message : String(reason));
      });
    return () => {
      live = false;
    };
  }, [prepared.scenario, setError]);
  if (!catalog)
    return (
      <LoadingScreen
        title="Deploying"
        subject="UPLOADED SCENARIO"
        stages={[{ id: "world", label: "Load bundle catalog" }]}
        current="world"
      />
    );
  const camera = gameCamera.fromStart(report.start.at, report.size);
  return (
    <SessionCatalogProvider catalog={catalog}>
      <BattleView
        fixture="generated"
        scenario={prepared.scenario}
        scenarioArt
        seed={1}
        prepared={prepared}
        replay={replayText}
        player={report.active_human_player ?? null}
        inCommand={!takenOver}
        side={report.active_human_side ?? "blue"}
        camera={camera}
        cameraConfig={gameCamera.forMap(report.size, report.extents.rendered)}
        spawn={report.start.at}
        cover={
          <LoadingScreen
            title="Deploying"
            subject="UPLOADED SCENARIO"
            stages={[
              { id: "bundle", label: "Validate package" },
              { id: "world", label: "Build world" },
            ]}
            current={stage}
          />
        }
        status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
        onDecoded={() => {
          const asset = report.assets[0];
          if (!asset || playedAsset.current === asset.id) return;
          playedAsset.current = asset.id;
          void appAudio
            .playEmbeddedAudio(asset.bytes_base64, asset.media_type)
            .catch((reason: unknown) => {
              setError(reason instanceof Error ? reason.message : String(reason));
            });
        }}
        menu={(battleSession) => (
          <>
            <button
              type="button"
              className="hud-menu-item"
              onClick={() =>
                void battleSession.sim.client?.replay().then((replay) => {
                  const text = JSON.stringify({ scenario: prepared.scenario, replay });
                  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
                  const anchor = document.createElement("a");
                  anchor.href = url;
                  anchor.download = "scenario-replay.json";
                  anchor.click();
                  setTimeout(() => URL.revokeObjectURL(url), 10_000);
                })
              }
            >
              Download replay
            </button>
            <label className="hud-menu-file">
              Upload replay
              <input
                type="file"
                accept="application/json,.json"
                onChange={async (event) => {
                  const file = event.target.files?.[0];
                  event.target.value = "";
                  if (!file) return;
                  try {
                    const parsed = JSON.parse(await file.text()) as {
                      scenario?: string;
                      replay?: string;
                    };
                    if (parsed.scenario !== prepared.scenario || typeof parsed.replay !== "string")
                      throw new Error("replay does not belong to this uploaded scenario");
                    setReplayText(parsed.replay);
                  } catch (reason) {
                    setError(reason instanceof Error ? reason.message : String(reason));
                  }
                }}
              />
            </label>
            {report.active_human_player && !takenOver && (
              <button
                type="button"
                className="hud-menu-item"
                onClick={() => {
                  battleSession.sim.client?.setControllers(
                    JSON.stringify([[report.active_human_player, "active_ai"]]),
                  );
                  setTakenOver(true);
                }}
              >
                Hand your player to the AI
              </button>
            )}
            <Link className="hud-menu-item" to="/labs/scenario?mode=play">
              Upload another scenario
            </Link>
          </>
        )}
      />
    </SessionCatalogProvider>
  );
}
