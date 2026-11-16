import { lightingChoices } from "@packages/skirmish-setup/src/lighting";
// One order of battle: fixed Blue/Red columns, the admitted map and settings.
// Drivers own mutations; this view renders their current seats and settings.
import { useMemo } from "react";
import { Link } from "react-router";
import { MAP_SIZES, MAP_TYPES } from "@web/maps/source";
import { REGIONS, type AskedLobby } from "../battleLinks";
import { lobbyPlayers, sizeFloor, type LobbyPreset } from "@packages/skirmish-setup/src/lobby";
import { battlePlayers } from "@packages/skirmish-setup/src/seats";
import { lightingLabel } from "./lighting";
import { Choice } from "./Choices";
import { holders, MapFrame, PreviewStatus } from "./MapFrame";
import { SideColumn } from "./Seats";
import { useLocalLobby } from "./localDriver";
import type { LobbyDriver } from "./driver";

const RANDOM = "random";
const PRESETS: readonly LobbyPreset[] = ["1v1", "3v3", "5v5"];

export function Lobby(props: { asked: AskedLobby } | { driver: LobbyDriver }) {
  return "asked" in props ? (
    <LocalLobby asked={props.asked} />
  ) : (
    <DrivenLobby driver={props.driver} />
  );
}
function LocalLobby({ asked }: { asked: AskedLobby }) {
  return <DrivenLobby driver={useLocalLobby(asked)} />;
}
function DrivenLobby({ driver }: { driver: LobbyDriver }) {
  const { lobby, preview, wasm } = driver;
  const local = driver.kind === "local" ? driver : null;
  const room = driver.kind === "room" ? driver : null;
  const count = (side: "blue" | "red") =>
    lobby.seats.filter((seat) => seat.side === side && seat.occupant.kind !== "open").length;
  const ready = preview.status === "ready";
  const sites = ready ? preview.preview.sites : null;
  const held = useMemo(() => {
    if (!wasm || !sites) return null;
    const players = lobbyPlayers(lobby);
    const you = room ? battlePlayers(lobby.seats).playerOf[room.view.you] : undefined;
    return holders(wasm, sites, players, you);
  }, [wasm, sites, lobby, room]);
  const disabled = room !== null && !room.editable;
  return (
    <section className="lobby" aria-label={room ? "Room lobby" : "New battle"}>
      <div className="lobby-field">
        <SideColumn side="blue" driver={driver} />
        <figure className="lobby-map">
          <figcaption className="lobby-vs" data-testid="lobby-vs">
            <span data-side="blue">{count("blue")}</span> VS{" "}
            <span data-side="red">{count("red")}</span>
          </figcaption>
          <MapFrame
            state={preview}
            held={held}
            failureHint={
              room ? (preview.status === "failed" ? preview.message : undefined) : undefined
            }
          />
          {room && <p className="online-open-note">Open seats sit out when the battle starts.</p>}
        </figure>
        <SideColumn side="red" driver={driver} />
      </div>
      <div className="lobby-strip">
        {disabled && (
          <span
            className="lobby-label online-settings-note"
            role="note"
            aria-label="Set by the host"
          >
            Set by the host
          </span>
        )}
        {local && (
          <Choice
            label="match"
            options={PRESETS}
            value={PRESETS.find((kind) => kind === `${count("blue")}v${count("red")}`) ?? null}
            onChange={local.preset}
          />
        )}
        <Choice
          label="map"
          options={MAP_TYPES}
          value={lobby.map.type}
          disabled={disabled}
          onChange={driver.setType}
        />
        <Choice
          label="size"
          options={MAP_SIZES}
          value={lobby.map.size}
          disabled={disabled}
          onChange={driver.setSize}
          closed={(size) => MAP_SIZES.indexOf(size) < MAP_SIZES.indexOf(sizeFloor(lobby))}
        />
        <Choice
          label="region"
          folds
          options={[RANDOM, ...REGIONS]}
          value={lobby.map.region ?? RANDOM}
          disabled={disabled}
          onChange={(region) => driver.setRegion(region === RANDOM ? undefined : region)}
        />
        <Choice
          label="lighting"
          folds
          options={lightingChoices()}
          value={lobby.lighting}
          disabled={disabled}
          onChange={driver.setLighting}
          say={lightingLabel}
        />
        <PreviewStatus state={preview} label={room?.previewStatus} />
        {(!room || room.editable) && (
          <button
            type="button"
            className="lobby-reroll lobby-order"
            data-testid="lobby-reroll"
            onClick={driver.reroll}
          >
            Reroll
          </button>
        )}
        {local ? (
          ready ? (
            <Link
              className="lobby-deploy"
              data-testid="menu-deploy"
              target="_self"
              to={local.deploy!}
            >
              Deploy
            </Link>
          ) : (
            <a className="lobby-deploy" data-testid="menu-deploy" role="link" aria-disabled="true">
              Deploy
            </a>
          )
        ) : (
          room && <RoomCommands driver={room} />
        )}
      </div>
    </section>
  );
}
function RoomCommands({ driver }: { driver: Extract<LobbyDriver, { kind: "room" }> }) {
  const own = driver.view.seats.find((seat) => seat.id === driver.view.you)?.occupant;
  const ready = own?.kind === "human" && own.ready;
  return (
    <>
      <button
        type="button"
        className="lobby-order"
        aria-pressed={ready}
        onClick={() => driver.ready(!ready)}
      >
        {ready ? "Unready" : "Ready"}
      </button>
      {driver.editable && (
        <button
          type="button"
          className="lobby-order online-start"
          disabled={!driver.view.canStart}
          onClick={driver.start}
        >
          Start
        </button>
      )}
    </>
  );
}
