// /lab/generated: a battle on a generated map. `?type=open|mixed|metro`,
// `?size=small|medium|large` and `?seed=<u64>` choose the map (mixed, small,
// 1 by default). A preparation worker generates it with the simulation's own
// generator and lays the developer encounter on it (`fixtures/generated-lab.json`);
// the battle then runs under the village's rules, as the village does.
import { useEffect, useMemo, useRef, useState } from "react";
import village from "@fixtures/village.json";
import lab from "@fixtures/generated-lab.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import type { CameraPresentation } from "@packages/renderer-core/src/cameraController";
import { prepareBattle, PreparationFailed } from "@web/battle/prepare/client";
import {
  MAP_SIZES,
  MAP_TYPES,
  type MapChoice,
  type PreparedBattle,
  type PrepareStage,
} from "@web/battle/prepare/protocol";
import { BattleView, type BattleLoadStage } from "../BattleView";
import { LoadingScreen, type LoadingStage } from "../LoadingScreen";
import { VILLAGE_RULES } from "../scenarios";
import { villageCamera } from "../villageCamera";

const DEFAULT: MapChoice = { type: "mixed", size: "small", seed: "1" };
const U64_MAX = 2n ** 64n - 1n;

/** The map the URL asks for, or which parameter it gets wrong. */
function urlChoice(search: string): MapChoice | { error: string } {
  const params = new URLSearchParams(search);
  const type = params.get("type") ?? DEFAULT.type;
  const size = params.get("size") ?? DEFAULT.size;
  const seed = params.get("seed") ?? DEFAULT.seed;
  const one = <T extends string>(value: string, of: readonly T[]): value is T =>
    (of as readonly string[]).includes(value);
  if (!one(type, MAP_TYPES)) return { error: `type must be one of ${MAP_TYPES.join(", ")}` };
  if (!one(size, MAP_SIZES)) return { error: `size must be one of ${MAP_SIZES.join(", ")}` };
  if (!/^(0|[1-9]\d{0,19})$/.test(seed) || BigInt(seed) > U64_MAX)
    return { error: "seed must be a whole number from 0 to 18446744073709551615" };
  return { type, size, seed };
}

type Stage = PrepareStage | "world" | "renderer";
const STAGES: readonly (LoadingStage & { id: Stage })[] = [
  { id: "generating", label: "Generating the map" },
  { id: "placing", label: "Placing forces" },
  { id: "world", label: "Building the battlefield" },
  { id: "renderer", label: "Starting the battle" },
];

/** The camera rig for a map `size` metres across: the village's, with the
 *  wheel reaching far enough out, and tilting far enough down, to take the
 *  whole map in. */
function mapCamera(size: [number, number]): CameraPresentation {
  const far = Math.max(villageCamera.config.zoom_max, Math.max(...size) * lab.camera.overview_span);
  return {
    ...villageCamera.config,
    zoom_max: far,
    pitch_curve: [...villageCamera.config.pitch_curve, [far, lab.camera.overview_pitch]],
  };
}

interface Failure {
  message: string;
  details: string[];
}

/** When each stage of this page's loading finished, in milliseconds since
 *  navigation started. */
type StartupMarks = Partial<Record<"prepared" | BattleLoadStage, number>>;

export default function GeneratedBattle() {
  const [choice] = useState(() => urlChoice(window.location.search));
  const [stage, setStage] = useState<Stage>("generating");
  const [prepared, setPrepared] = useState<PreparedBattle | null>(null);
  const [failure, setFailure] = useState<Failure | null>(
    "error" in choice
      ? { message: "This map cannot be requested.", details: [choice.error] }
      : null,
  );
  const marks = useRef<StartupMarks>({});

  useEffect(() => {
    if ("error" in choice) return;
    const preparation = prepareBattle(
      {
        type: "prepare",
        map: choice,
        presets,
        templates,
        limits: lab.limits,
        rules: JSON.stringify(VILLAGE_RULES),
        encounter: lab.encounter,
      },
      setStage,
    );
    preparation.battle.then(
      (battle) => {
        marks.current.prepared = performance.now();
        setStage("world");
        setPrepared(battle);
      },
      (error: unknown) =>
        setFailure({
          message: "This map could not be generated.",
          details:
            error instanceof PreparationFailed && error.diagnostics.length
              ? error.diagnostics.map((d) => `${d.code} at ${d.location}: ${d.message}`)
              : [error instanceof Error ? error.message : String(error)],
        }),
    );
    return () => preparation.cancel();
  }, [choice]);

  const subject =
    "error" in choice
      ? "Generated map"
      : `${choice.type} · ${choice.size} · seed ${choice.seed}`.toUpperCase();
  const cover = (
    <LoadingScreen
      title="Deploying"
      subject={subject}
      stages={STAGES}
      current={stage}
      failure={failure}
    />
  );
  const view = useMemo(
    () =>
      prepared && {
        camera: {
          ...villageCamera.opening(),
          target: [prepared.report.anchors.blue[0], prepared.report.anchors.blue[1], 0] as [
            number,
            number,
            number,
          ],
        },
        cameraConfig: mapCamera(prepared.report.size),
      },
    [prepared],
  );
  if (!prepared || !view) return cover;
  return (
    <BattleView
      fixture="generated"
      scenario={prepared.scenario}
      seed={village.seed}
      camera={view.camera}
      cameraConfig={view.cameraConfig}
      cover={cover}
      onLoadStage={(loaded) => {
        marks.current[loaded] ??= performance.now();
        if (loaded === "world") setStage("renderer");
      }}
      status={({ sim }) => {
        const s = (sim.observation?.tick ?? 0) / village.tick_hz;
        return (
          <>
            <span className="hud-objective" data-testid="map">
              {subject}
            </span>
            <span className="hud-clock" data-testid="clock">
              {Math.floor(s / 60)}:{String(Math.floor(s % 60)).padStart(2, "0")}
            </span>
          </>
        );
      }}
      diagnostics={() => ({
        /** What the preparation worker made: the map's identity, its counts,
         *  where the encounter stands and what each stage cost. */
        generated: () => prepared.report,
        /** When each loading stage finished, ms since navigation started. */
        startup: () => ({ ...marks.current }),
      })}
    />
  );
}
