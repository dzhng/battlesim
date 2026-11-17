import { FRONTLINE_FAILED_NOTICE, FRONTLINE_IDLE_NOTICE } from "../online/frontlineRequests";
import { ROOM_BATTLE_FAILED_NOTICE } from "../online/requests";
import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router";
import type { BattleConnectionSession } from "@web/battle/prepare/client";
import type { Player, PreparedBattle } from "@web/battle/prepare/protocol";
import { lanesOf } from "@web/battle/prepare/lanes";
import { loadWasm } from "@web/battle/sim/module";
import { httpLink } from "@web/battle/remote/httpLink";
import { remoteConnection } from "@web/battle/remote/connection";
import { streamEnded } from "@web/battle/remote/streamEnd";
import { useOnlineMap, type OnlineMap } from "../online/useOnlineMap";
import { useOnlineBattlePresence } from "../online/battlePresence";
import { BattleView } from "../BattleView";
import { BattleClock } from "../battleStatus";
import { LoadingScreen } from "../LoadingScreen";
import {
  builtMap,
  returnToMultiplayer,
  leaveOnlineBattle,
  type OnlineBattleEntry,
} from "../onlineSession";
import { browserId } from "../online/identity";
import { gameCamera } from "../gameCamera";
import { resolveLighting } from "../lobby/lighting";
import { setMatchLighting } from "../gameLight";
import { usePageVisitActive } from "../navigation";

function Cover({
  stage,
  error,
  onLeave,
}: {
  stage: string;
  error: string | null;
  onLeave?: () => void;
}) {
  const location = useLocation();
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
      back={`/multiplayer${location.search}`}
      onCancel={onLeave}
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
    return <Cover stage="map" error="This page needs a battle ticket. Join through Multiplayer." />;
  return <TicketBattle entry={entry} />;
}

function TicketBattle({ entry }: { entry: OnlineBattleEntry }) {
  useOnlineBattlePresence(entry);
  const built = builtMap(entry.ticket.battle);
  if (built) return <PreparedOnlineBattle entry={entry} prepared={built} />;
  return <PrepareTicket entry={entry} />;
}

function PrepareTicket({ entry }: { entry: OnlineBattleEntry }) {
  const map = useOnlineMap(entry.ticket.setup, entry.ticket.mapIdentity);
  if (map.status !== "ready")
    return (
      <Cover
        onLeave={() => {
          leaveOnlineBattle(entry);
        }}
        stage={map.status === "preparing" ? map.stage : "map"}
        error={map.status === "failed" ? map.error : null}
      />
    );
  return <PreparedOnlineBattle entry={entry} prepared={map.prepared} />;
}

function PreparedOnlineBattle({
  entry,
  prepared: { request, battle },
}: {
  entry: OnlineBattleEntry;
  prepared: OnlineMap;
}) {
  const { ticket, address } = entry;
  const navigate = useNavigate();
  const location = useLocation();
  const active = usePageVisitActive();
  const [stage, setStage] = useState("map");
  const [error, setError] = useState<string | null>(null);
  const [world, setWorld] = useState<{
    battle: PreparedBattle;
    session: BattleConnectionSession;
  } | null>(null);
  useEffect(() => {
    let live = true;
    void (async () => {
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
          connect: remoteConnection(httpLink(ticket.address, ticket.battle, browserId())),
        },
      });
      setStage("world");
    })().catch((reason: Error) => {
      if (live && active()) setError(reason.message);
    });
    return () => {
      live = false;
    };
  }, [ticket, address, battle, active]);
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
  const leave = () => leaveOnlineBattle(entry);
  const back = (notice?: string) =>
    returnToMultiplayer(navigate, location.search, ticket.setup.rules, notice);
  const cover = <Cover stage={stage} error={error} onLeave={leave} />;
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
      onLeave={() => {
        leave();
        back();
      }}
      onFailure={(message) =>
        back(
          message === streamEnded("idle")
            ? FRONTLINE_IDLE_NOTICE
            : ticket.setup.rules === "frontline"
              ? FRONTLINE_FAILED_NOTICE
              : ROOM_BATTLE_FAILED_NOTICE,
        )
      }
      callsigns={ticket.callsigns}
      onLoadStage={(loaded) => {
        if (loaded === "world") setStage("renderer");
      }}
      status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
      diagnostics={() => ({ prepared: () => world.battle.report })}
    />
  );
}
