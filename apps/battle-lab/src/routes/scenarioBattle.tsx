// /battle?scenario=<map>/<name>: an authored scenario's battle (the
// tutorial is one), prepared from its saved map, document and coach under
// this build's rules, in the battle view every battle plays in.
import { useEffect, useMemo, useRef, useState } from "react";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { prepareBattle, PreparationFailed, type PreparedSession } from "@web/battle/prepare/client";
import type { PrepareStage, ScenarioRequest } from "@web/battle/prepare/protocol";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { usePageVisitActive } from "../navigation";
import { setMatchLighting } from "../gameLight";
import { DEFAULT_LIGHTING, resolveLighting } from "../lobby/lighting";
import { BattleClock } from "../battleStatus";
import { BattleView } from "../BattleView";
import { LoadingScreen, type LoadingFailure, type LoadingStage } from "../LoadingScreen";
import { gameCamera } from "../gameCamera";

type Stage = PrepareStage | "world" | "renderer";
const STAGES: (LoadingStage & { id: Stage })[] = [
  { id: "map", label: "Loading the map" },
  { id: "encounter", label: "Placing forces" },
  { id: "world", label: "Building the battlefield" },
  { id: "renderer", label: "Starting the battle" },
];

/** What the loading screen calls the battle: its map and scenario. */
const subjectOf = ({ map, name }: ScenarioRequest) =>
  `${map} · ${name}`.replaceAll("-", " ").toUpperCase();

function failureOf(error: unknown): LoadingFailure {
  const details =
    error instanceof PreparationFailed && error.diagnostics.length
      ? error.diagnostics.map((d) => `${d.code} at ${d.location}: ${d.message}`)
      : [error instanceof Error ? error.message : String(error)];
  return { message: "This scenario could not be prepared.", details };
}

export function ScenarioBattleView({ scenario }: { scenario: ScenarioRequest }) {
  const visitActive = usePageVisitActive();
  const { rules } = useSessionCatalog();
  const [stage, setStage] = useState<Stage>("map");
  const [prepared, setPrepared] = useState<PreparedSession | null>(null);
  const [failure, setFailure] = useState<LoadingFailure | null>(null);
  const { map, name, chapter, beat } = scenario;

  useEffect(() => {
    setMatchLighting({ preset: resolveLighting(DEFAULT_LIGHTING, map), seed: map });
    return () => setMatchLighting(null);
  }, [map]);

  useEffect(() => {
    let live = true;
    setPrepared(null);
    setFailure(null);
    setStage("map");
    const documents = { rules: JSON.stringify(rules), presets, templates };
    const preparation = prepareBattle(
      { type: "prepare-scenario", scenario: { map, name, chapter, beat }, documents },
      setStage,
    );
    preparation.battle.then(
      (battle) => {
        if (!live || !visitActive()) return;
        setStage("world");
        setPrepared(battle);
      },
      (error: unknown) => {
        if (live && visitActive()) setFailure(failureOf(error));
      },
    );
    return () => {
      live = false;
      preparation.cancel();
    };
  }, [map, name, chapter, beat, visitActive, rules]);

  const subject = subjectOf(scenario);
  const cover = (
    <LoadingScreen
      title="Deploying"
      subject={subject}
      stages={STAGES}
      current={stage}
      failure={failure}
      back="/"
    />
  );
  const view = useMemo(
    () =>
      prepared && {
        // The scenario's person: its human-capable seat, even where a battle
        // started at a chapter hands that seat back only at its resume.
        player: (
          JSON.parse(prepared.scenario) as { players?: { id: string; human_capable?: boolean }[] }
        ).players?.find((seat) => seat.human_capable)?.id,
        camera: gameCamera.fromStart(prepared.report.start.at, prepared.report.size),
        cameraConfig: gameCamera.forMap(prepared.report.size, prepared.report.extents.rendered),
      },
    [prepared],
  );
  const marks = useRef<Partial<Record<string, number>>>({});
  if (!prepared || !view || !("scenario" in prepared.report)) return cover;
  const { report } = prepared;
  return (
    <BattleView
      fixture="generated"
      prepared={prepared}
      scenario={prepared.scenario}
      seed={report.scenario.battleSeed}
      player={view.player}
      camera={view.camera}
      cameraConfig={view.cameraConfig}
      cover={cover}
      onFailure={(error) =>
        setFailure({ message: "The battle could not be started.", details: [error] })
      }
      onLoadStage={(loaded) => {
        marks.current[loaded] ??= performance.now();
        if (loaded === "world") setStage("renderer");
      }}
      status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
      coach={report.coach ?? undefined}
      diagnostics={() => ({
        /** What the preparation worker made: the scenario, the map's
         *  identity and counts, and what each stage cost. */
        prepared: () => report,
        /** When each loading stage finished, ms since navigation started. */
        startup: () => ({ ...marks.current }),
      })}
    />
  );
}
