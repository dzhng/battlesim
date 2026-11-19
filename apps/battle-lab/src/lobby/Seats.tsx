import { FACTIONS } from "@packages/scene-assets/src/units";
import { sideFaction, type LobbySide } from "@packages/skirmish-setup/src/lobby";
import {
  battlePlayers,
  MAX_PLAYERS_PER_SIDE,
  type LobbySeat,
} from "@packages/skirmish-setup/src/seats";
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
  // battlePlayers requires both sides. For numbering an unfinished room only,
  // give an empty side a placeholder; it is never rendered or sent to a preview.
  const numbering = [...lobby.seats];
  if (room)
    for (const team of ["blue", "red"] as const)
      if (!numbering.some((seat) => seat.side === team && seat.occupant.kind !== "open"))
        numbering.push({ id: `numbering-${team}`, side: team, occupant: { kind: "ai" } });
  const { playerOf } = battlePlayers(numbering);
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
            player={playerOf[seat.id]}
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
  seat: LobbySeat;
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
  const sideName = seat.side === "blue" ? "Blue" : "Red";
  const name = `${sideName} ${index}`;
  const lane = player?.split("-").at(-1);
  const removable = count > 1 && (room ? room.editable && !you : occupant.kind !== "human");
  /** What holds a seat nobody sits in. */
  const vacant = occupant.kind === "human" ? null : occupant.kind;
  const sit = () => driver.sit(seat.id);
  const removeSeat = () => driver.remove(seat.id);
  return (
    <li
      className="lobby-banner"
      aria-label={room ? `${name} seat` : undefined}
      data-player={player}
      data-you={you || undefined}
      data-kind={room ? occupant.kind : undefined}
    >
      <span
        className="lobby-index"
        aria-hidden={room ? undefined : true}
        role={room && player ? "img" : undefined}
        aria-label={room && player ? `${sideName} lane ${lane}` : undefined}
      >
        {room ? lane : index}
      </span>
      {room ? (
        <>
          {occupant.kind === "human" && "callsign" in occupant ? (
            <span className="lobby-controller online-human">
              {occupant.callsign}
              {you ? " · you" : ""}
              {occupant.host && <small>host</small>}
            </span>
          ) : room.editable && vacant ? (
            <Pick
              label={`${name} seat`}
              testId={`lobby-seat-${seat.id}`}
              className="lobby-controller"
              options={vacant === "open" ? ["open", "ai", "move_here"] : ["open", "ai"]}
              value={vacant}
              onChange={(to) => {
                if (to === "move_here") sit();
                else room.setSeat(seat.id, to);
              }}
            >
              {vacant === "open" ? "Open" : "AI"}
            </Pick>
          ) : (
            <>
              <span className="lobby-controller">{vacant === "open" ? "Open" : "AI"}</span>
              {vacant === "open" && (
                <button
                  type="button"
                  className="lobby-move"
                  aria-label={`Move here, ${name}`}
                  onClick={sit}
                >
                  Move here
                </button>
              )}
            </>
          )}
          {occupant.kind === "human" && "ready" in occupant && occupant.ready && (
            <span className="online-ready" role="status" aria-label={`${occupant.callsign} ready`}>
              Ready
            </span>
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
          aria-label={
            room && occupant.kind === "human" && "callsign" in occupant
              ? `Remove ${occupant.callsign} from room`
              : `Remove ${name}`
          }
          data-testid={`lobby-remove-${player}`}
          onClick={removeSeat}
        />
      )}
    </li>
  );
}
