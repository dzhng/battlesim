import { FACTIONS } from "@packages/scene-assets/src/units";
import { sideFaction, lobbyPlayers, type LobbySide } from "@packages/skirmish-setup/src/lobby";
import { MAX_PLAYERS_PER_SIDE, type Seat } from "@packages/skirmish-setup/src/seats";
import { spoken } from "../battleLinks";
import { Pick } from "./Choices";
import type { LobbyDriver } from "./driver";

/** Fixed sides, the driver's faction, and the same banner for every seat. */
export function SideColumn({ side, driver }: { side: LobbySide; driver: LobbyDriver }) {
  const { lobby } = driver;
  const room = driver.kind === "room" ? driver : null;
  const seats = lobby.seats.filter(
    (seat) => seat.side === side && (room || seat.occupant.kind !== "open"),
  );
  const players = room ? [] : lobbyPlayers(lobby).filter((player) => player.side === side);
  const name = side === "blue" ? "Blue" : "Red";
  return (
    <div className="lobby-side" data-side={side} aria-label={`${name} side`} role="group">
      <Pick
        label={`${name} faction`}
        testId={`lobby-faction-${side}`}
        className="lobby-faction"
        options={FACTIONS}
        value={sideFaction(lobby, side)}
        disabled={room !== null && !room.editable}
        onChange={(faction) => driver.setFaction(side, faction)}
      >
        {name} · {spoken(sideFaction(lobby, side))}
      </Pick>
      <ol className="lobby-banners">
        {seats.map((seat, index) => (
          <Banner
            key={seat.id}
            seat={seat}
            index={index + 1}
            player={players[index]?.id}
            count={seats.length}
            driver={driver}
          />
        ))}
      </ol>
      {seats.length < MAX_PLAYERS_PER_SIDE && (!room || room.editable) && (
        <button
          type="button"
          className="lobby-add"
          data-testid={`lobby-add-${side}`}
          onClick={() => driver.add(side)}
        >
          {room ? "Add seat" : "Add player"}
        </button>
      )}
    </div>
  );
}

function Banner({
  seat,
  index,
  player,
  count,
  driver,
}: {
  seat: Seat;
  index: number;
  player?: string;
  count: number;
  driver: LobbyDriver;
}) {
  const room = driver.kind === "room" ? driver : null;
  const occupant = room
    ? room.view.seats.find((view) => view.id === seat.id)!.occupant
    : seat.occupant;
  const you = room ? room.view.you === seat.id : occupant.kind === "human";
  const name = `${seat.side === "blue" ? "Blue" : "Red"} ${index}`;
  const removable = count > 1 && (room ? room.editable && !you : occupant.kind !== "human");
  const sit = () => driver.sit(seat.id);
  const removeSeat = () => driver.remove(seat.id);
  return (
    <li
      className="lobby-banner"
      data-player={player}
      data-you={you || undefined}
      data-kind={room ? occupant.kind : undefined}
    >
      <span className="lobby-index" aria-hidden>
        {index}
      </span>
      {room ? (
        <>
          {occupant.kind === "human" && "callsign" in occupant ? (
            <span className="lobby-controller online-human">
              {occupant.callsign}
              {you ? " · you" : ""}
              {occupant.host && <small>host</small>}
            </span>
          ) : occupant.kind === "open" ? (
            <button
              type="button"
              className="lobby-controller online-sit"
              aria-label={`Sit ${name}`}
              onClick={sit}
            >
              Open
            </button>
          ) : (
            <span className="lobby-controller">AI</span>
          )}
          {occupant.kind === "human" && "ready" in occupant && occupant.ready && (
            <span className="online-ready" role="status" aria-label={`${occupant.callsign} ready`}>
              Ready
            </span>
          )}
          {room.editable && occupant.kind !== "human" && (
            <button
              type="button"
              className="online-set-seat"
              aria-label={`Set ${name} ${occupant.kind === "open" ? "AI" : "open"}`}
              onClick={() => room.setSeat(seat.id, occupant.kind === "open" ? "ai" : "open")}
            >
              {occupant.kind === "open" ? "Set AI" : "Set open"}
            </button>
          )}
        </>
      ) : seat.side === "blue" && !you ? (
        <Pick
          label={`${name} controller`}
          testId={`lobby-controller-${player}`}
          className="lobby-controller"
          options={["ai", "you"] as const}
          value="ai"
          onChange={(choice) => choice === "you" && sit()}
        >
          AI
        </Pick>
      ) : (
        <span className="lobby-controller" data-testid={`lobby-controller-${player}`}>
          {you ? "You" : "AI"}
        </span>
      )}
      {removable && (
        <button
          type="button"
          className="lobby-remove"
          aria-label={`Remove ${name}`}
          data-testid={`lobby-remove-${player}`}
          onClick={removeSeat}
        />
      )}
    </li>
  );
}
