import { returnToMultiplayer } from "../onlineSession";
import { useCallback, useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router";
import type { RoomAction, RoomView } from "@packages/multiplayer/src/wire";
import { PageHead } from "../menu/PageHead";
import { PlateStatus } from "../menu/PlateStatus";
import { Lobby } from "../lobby/Lobby";
import { useRoom, useRoomDriver } from "./roomDriver";
import "./online.css";

export function RoomPage() {
  const location = useLocation();
  const navigate = useNavigate();
  const code = decodeURIComponent(location.pathname.split("/").at(-1)!);
  const room = useRoom(code);
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState(false);
  const leave = useCallback(() => {
    if (room.view) room.act({ kind: "leave" });
    else returnToMultiplayer(navigate, location.search);
  }, [room.view, room.act, navigate, location.search]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") leave();
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [leave]);
  return (
    <>
      <PageHead title="Room lobby" onBack={leave}>
        {room.view && (
          <div className="online-room-header">
            <span>{room.view.visibility}</span>
            <strong>{room.view.code}</strong>
            <button
              type="button"
              onClick={() => {
                const link = new URL(
                  `/room/${encodeURIComponent(room.view!.code)}`,
                  window.location.origin,
                ).href;
                void navigator.clipboard.writeText(link).then(
                  () => {
                    setCopied(true);
                    setCopyError(false);
                  },
                  () => setCopyError(true),
                );
              }}
            >
              {copied ? "Copied" : "Copy link"}
            </button>
            <button type="button" onClick={leave}>
              Leave
            </button>
          </div>
        )}
      </PageHead>
      {room.error && <p role="alert">{room.error}</p>}
      {copyError && <p role="alert">Could not copy the link.</p>}
      {room.unavailable ? (
        <PlateStatus label="Online unavailable" detail={room.unavailable} failed />
      ) : !room.view ? (
        <PlateStatus label="Joining room" />
      ) : room.view.phase.kind !== "lobby" ? (
        <PlateStatus label="Preparing the battlefield" detail="This can take a few seconds." />
      ) : (
        <RoomLobby view={room.view} act={room.act} />
      )}
    </>
  );
}
function RoomLobby({ view, act }: { view: RoomView; act: (action: RoomAction) => void }) {
  return <Lobby driver={useRoomDriver(view, act)} />;
}
