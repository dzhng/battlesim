import { useEffect, useMemo, useState } from "react";
import { useLocation } from "react-router";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { fromBattleSetup } from "@packages/multiplayer/src/setup";
import { battleRequest } from "@packages/skirmish-setup/src/lobby";
import { prepareBattle, type BattleConnectionSession } from "@web/battle/prepare/client";
import type { Player, PrepareBattleRequest, PreparedBattle } from "@web/battle/prepare/protocol";
import { lanesOf } from "@web/battle/prepare/lanes";
import { loadWasm } from "@web/battle/sim/module";
import { httpLink } from "@web/battle/remote/httpLink";
import { remoteConnection } from "@web/battle/remote/connection";
import { BattleView } from "../BattleView";
import { BattleClock } from "../battleStatus";
import { LoadingScreen } from "../LoadingScreen";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { useBuiltScenario, buildFailed } from "../useBuiltScenario";
import { setupData } from "../skirmishData";
import type { OnlineBattleEntry } from "../onlineSession";
import { browserId } from "../online/identity";
import { gameCamera } from "../gameCamera";
import { resolveLighting } from "../lobby/lighting";
import { setMatchLighting } from "../gameLight";
import { usePageVisitActive } from "../navigation";

function Cover({ stage, error }: { stage: string; error: string | null }) {
  return (
    <LoadingScreen
      title="Deploying"
      subject="ONLINE BATTLE"
      stages={[
        { id: "map", label: "Generating the map" },
        { id: "encounter", label: "Placing forces" },
        { id: "world", label: "Building the battlefield" },
        { id: "renderer", label: "Joining the battle" },
      ]}
      current={stage}
      failure={error ? { message: "The battle could not be started.", details: [error] } : null}
    />
  );
}

/** A ticket describes the world and seat; it never contains the credential. */
export default function OnlineBattle() {
  const location = useLocation();
  const entry = location.state as OnlineBattleEntry | null;
  if (!entry || location.pathname !== `/online/battle/${encodeURIComponent(entry.ticket.battle)}`)
    return <Cover stage="map" error="This page needs a battle ticket. Join through /online/dev." />;
  return <TicketBattle entry={entry} />;
}

function TicketBattle({ entry }: { entry: OnlineBattleEntry }) {
  const request = useBuiltScenario(entry.ticket.setup, (wasm, setup) =>
    battleRequest(fromBattleSetup(setup), setupData(wasm)),
  );
  if (!request) return <Cover stage="map" error={null} />;
  if (buildFailed(request)) return <Cover stage="map" error={request.error} />;
  return <PreparedOnlineBattle entry={entry} request={request} />;
}

function PreparedOnlineBattle({
  entry: { ticket, address },
  request,
}: {
  entry: OnlineBattleEntry;
  request: PrepareBattleRequest;
}) {
  const { rules } = useSessionCatalog();
  const active = usePageVisitActive();
  const [stage, setStage] = useState("map");
  const [error, setError] = useState<string | null>(null);
  const [world, setWorld] = useState<{
    battle: PreparedBattle;
    session: BattleConnectionSession;
  } | null>(null);
  useEffect(() => {
    let live = true;
    const preparation = prepareBattle(
      {
        type: "prepare",
        request,
        documents: {
          rules: JSON.stringify(rules),
          presets,
          templates,
        },
      },
      setStage,
    );
    void preparation.battle
      .then(async (battle) => {
        // This check precedes even constructing the remote session. A mismatched
        // page must never observe a single record from the server.
        if (
          battle.report.identity.kind !== "generated" ||
          battle.report.identity.generation.map_hash !== ticket.mapIdentity
        )
          throw new Error("The local map differs from the server's map.");
        const wasm = await loadWasm();
        const scenario = JSON.parse(battle.scenario) as {
          players: Player[];
          skirmish: { sites: { entries: { center: [number, number]; yaw: number }[] } };
        };
        const sites = scenario.skirmish.sites;
        const lanes = lanesOf(wasm, JSON.stringify(sites), scenario.players);
        const home = lanes.entries.findIndex((lane) => lane.holder === ticket.player);
        if (home < 0) throw new Error("The commanding player has no entry road.");
        if (!live || !active()) return;
        setWorld({
          battle: {
            scenario: battle.scenario,
            report: {
              ...battle.report,
              start: { at: sites.entries[home].center, yaw: sites.entries[home].yaw },
            },
          },
          session: {
            transport: "remote",
            connect: remoteConnection(httpLink(address, ticket.battle, browserId())),
          },
        });
        setStage("world");
      })
      .catch((reason: Error) => {
        if (live && active()) setError(reason.message);
      })
      .finally(() => preparation.cancel());
    return () => {
      live = false;
      preparation.cancel();
    };
  }, [ticket, address, request, rules, active]);
  useEffect(() => {
    const seed = String(request.map_source.request.seed);
    setMatchLighting({ preset: resolveLighting(ticket.setup.lighting, seed), seed });
    return () => setMatchLighting(null);
  }, [ticket, request]);
  const view = useMemo(
    () =>
      world && {
        camera: gameCamera.fromStart(world.battle.report.start.at, world.battle.report.size),
        config: gameCamera.forMap(world.battle.report.size, world.battle.report.extents.rendered),
      },
    [world],
  );
  const cover = <Cover stage={stage} error={error} />;
  if (!world || !view || error) return cover;
  return (
    <BattleView
      fixture="online-battle"
      scenario={world.battle.scenario}
      seed={request.battle_seed}
      prepared={world.session}
      side={ticket.side}
      player={ticket.player}
      camera={view.camera}
      cameraConfig={view.config}
      cover={cover}
      onFailure={setError}
      onLoadStage={(loaded) => {
        if (loaded === "world") setStage("renderer");
      }}
      status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
      diagnostics={() => ({ prepared: () => world.battle.report })}
    />
  );
}
