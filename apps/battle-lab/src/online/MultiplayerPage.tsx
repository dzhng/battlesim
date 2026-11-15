import { useEffect, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router";
import type { RoomList } from "@packages/multiplayer/src/wire";
import { spoken } from "../battleLinks";
import { PageHead } from "../menu/PageHead";
import { useMenuShell } from "../menu/MenuShell";
import { callsign, setCallsign } from "./identity";
import { hello } from "./hello";
import { serverAddress } from "./server";
import { joinRoom, stateStream } from "./stateStream";
import { roomFailure, roomRequest } from "./roomRequests";
import "./online.css";

export function MultiplayerPage() {
  const { back } = useMenuShell();
  const navigate = useNavigate();
  const location = useLocation();
  const [address] = useState(() => serverAddress(window.location));
  const [name, setName] = useState(callsign);
  const assignedName = useRef(name);
  const [list, setList] = useState<RoomList | null>(null);
  const [available, setAvailable] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(
    () => (location.state as { roomNotice?: string } | null)?.roomNotice ?? null,
  );
  const [create, setCreate] = useState(false);
  const [joining, setJoining] = useState(false);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const lifetime = useRef<AbortController | null>(null);
  useEffect(() => {
    const visit = new AbortController();
    lifetime.current = visit;
    if (!address) setAvailable(false);
    else
      void hello(address, visit.signal)
        .then((assigned) => {
          if (visit.signal.aborted) return;
          assignedName.current = assigned;
          setName(assigned);
          setAvailable(true);
          stateStream<RoomList>(
            address,
            "/rooms",
            visit.signal,
            setList,
            () => setError("The room service has closed."),
            () => setAvailable(false),
          );
        })
        .catch(() => {
          if (!visit.signal.aborted) setAvailable(false);
        });
    return () => visit.abort();
  }, [address]);
  const run = async (action: (signal: AbortSignal) => Promise<void>) => {
    const signal = lifetime.current!.signal;
    setBusy(true);
    setError(null);
    try {
      await action(signal);
    } catch (error) {
      if (!signal.aborted) setError(roomFailure(error));
    } finally {
      if (!signal.aborted) setBusy(false);
    }
  };
  const enter = (room: string) =>
    run(async (signal) => {
      await joinRoom(address, room, signal);
      if (!signal.aborted) navigate(`/room/${encodeURIComponent(room)}${location.search}`);
    });
  const make = (visibility: "public" | "private") =>
    run(async (signal) => {
      const room = await roomRequest<{ code: string }>(address, "/rooms", { visibility }, signal);
      if (!signal.aborted) navigate(`/room/${encodeURIComponent(room.code)}${location.search}`);
    });
  return (
    <>
      <PageHead title="Multiplayer" onBack={back}>
        {available && (
          <form
            className="online-callsign"
            aria-label="Callsign"
            onSubmit={(event) => {
              event.preventDefault();
              if (busy || name === assignedName.current) return;
              void run(async (signal) => {
                setCallsign(name);
                const assigned = await hello(address, signal);
                if (!signal.aborted) {
                  assignedName.current = assigned;
                  setName(assigned);
                }
              });
            }}
          >
            <label>
              Callsign{" "}
              <input
                aria-label="Callsign"
                value={name}
                onChange={(event) => setName(event.target.value)}
                onBlur={(event) => event.currentTarget.form?.requestSubmit()}
              />
            </label>
          </form>
        )}
      </PageHead>
      {available === false ? (
        <p role="status" aria-label="Online unavailable">
          Online play is not available on this build.
        </p>
      ) : available === null ? (
        <p role="status">Connecting…</p>
      ) : (
        <section className="online-rooms" aria-label="Public rooms">
          <div className="online-room-tools">
            <h3>Public rooms</h3>
            <span className="online-seat-key">
              <SeatStrip seats={["human"]} side="blue" />
              Player <SeatStrip seats={["open"]} side="blue" />
              Open <SeatStrip seats={["ai"]} side="blue" />
              AI
            </span>
            <span className="online-spacer" />
            <button type="button" onClick={() => setJoining(!joining)}>
              Join private room
            </button>
            <button type="button" onClick={() => setCreate(!create)}>
              Create room
            </button>
          </div>
          {joining && (
            <form
              className="online-room-tools"
              onSubmit={(event) => {
                event.preventDefault();
                void enter(code.trim().toUpperCase());
              }}
            >
              <label>
                Room code{" "}
                <input
                  aria-label="Room code"
                  value={code}
                  onChange={(event) => setCode(event.target.value)}
                  autoFocus
                />
              </label>
              <button type="submit" disabled={busy || !code.trim()}>
                Enter room
              </button>
            </form>
          )}
          {create && (
            <div className="online-room-tools" role="group" aria-label="Room visibility">
              <button type="button" disabled={busy} onClick={() => void make("public")}>
                Public room
              </button>
              <button type="button" disabled={busy} onClick={() => void make("private")}>
                Private room
              </button>
            </div>
          )}
          {error && <p role="alert">{error}</p>}
          {list === null ? (
            <p role="status">Finding rooms…</p>
          ) : list.rooms.length === 0 ? (
            <p role="status" aria-label="No public rooms">
              No public rooms. Create one and call in your team.
            </p>
          ) : (
            <ul className="online-room-list" aria-label="Rooms">
              {list.rooms.map((room) => {
                const open = [...room.blue, ...room.red].includes("open");
                return (
                  <li
                    key={room.code}
                    className="online-room-row"
                    aria-label={`${room.host}'s room`}
                  >
                    <strong>{room.host}'s room</strong>
                    <span>
                      {spoken(room.map.type)} · {spoken(room.map.size)}
                      {room.map.region ? ` · ${spoken(room.map.region)}` : ""}
                    </span>
                    <span className="online-room-seats">
                      <SeatStrip seats={room.blue} side="blue" /> VS{" "}
                      <SeatStrip seats={room.red} side="red" />
                    </span>
                    {open ? (
                      <button
                        type="button"
                        disabled={busy}
                        aria-label={`Join ${room.host}'s room`}
                        onClick={() => void enter(room.code)}
                      >
                        Join
                      </button>
                    ) : (
                      <span>Full</span>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      )}
    </>
  );
}

function SeatStrip({
  seats,
  side,
}: {
  seats: RoomList["rooms"][number]["blue"];
  side: "blue" | "red";
}) {
  return (
    <span
      className="online-seat-strip"
      data-side={side}
      aria-label={`${side}: ${seats.join(", ")}`}
    >
      {seats.map((kind, index) => (
        <i key={index} data-kind={kind} />
      ))}
    </span>
  );
}
